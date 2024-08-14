use super::{
    BasicBlock, BasicBlockIdx, Body, Local, LocalDecl, Mutability, Operand, Place,
    Span, Statement, Terminator, TerminatorKind, Ty, UnwindAction, VarDebugInfo,
};

/// BodyMutator combines the data of the function body
/// with "ghost" basic block and local data, allowing the user
/// to instrument the original body with instructions in the
/// "ghost" section while iterating over the original data from the
/// function body.
pub struct BodyMutator {
    blocks: Vec<BasicBlock>,
    locals: Vec<LocalDecl>,
    arg_count: usize,
    var_debug_info: Vec<VarDebugInfo>,
    spread_arg: Option<Local>,
    span: Span,

    ghost_locals: Vec<LocalDecl>,
    ghost_blocks: Vec<BasicBlock>,
    ghost_statements: Vec<Statement>,
}

impl BodyMutator {
    /// Instantiate the body mutator
    pub fn new(
        blocks: Vec<BasicBlock>,
        locals: Vec<LocalDecl>,
        arg_count: usize,
        var_debug_info: Vec<VarDebugInfo>,
        spread_arg: Option<Local>,
        span: Span,
        ghost_locals: Vec<LocalDecl>,
        ghost_blocks: Vec<BasicBlock>,
        statements: Vec<Statement>,
    ) -> Self {
        BodyMutator {
            blocks,
            locals,
            arg_count,
            var_debug_info,
            spread_arg,
            span,
            ghost_locals,
            ghost_blocks,
            ghost_statements: statements,
        }
    }

    /// Generate bb0 which jumps to the "ghost" basic blocks
    pub fn gen_bb0(body: &mut Body) -> BasicBlock {
        let target = body.blocks.len() + 1;
        let kind = TerminatorKind::Goto { target };
        let span = body.span;
        let terminator = Terminator { kind, span };
        let statements = Vec::new();
        std::mem::replace(&mut body.blocks[0], BasicBlock { statements, terminator })
    }

    /// Generate a unit local variable to be
    /// used as the destination of function calls
    pub fn gen_unit(body: &Body) -> LocalDecl {
        let ty = Ty::new_tuple(&[]);
        let span = body.span;
        let mutability = Mutability::Not;
        LocalDecl { ty, span, mutability }
    }

    /// Create this body from Mir's Body
    pub fn from(mut body: Body) -> Self {
        let bb0 = Self::gen_bb0(&mut body);
        body.blocks.push(bb0);
        let ghost_locals = vec![Self::gen_unit(&body)];
        let ghost_blocks = vec![];
        let locals = body.locals().to_vec();
        let arg_count = body.arg_locals().len();
        let spread_arg = body.spread_arg();
        let debug_info = body.var_debug_info;
        let statements = Vec::new();
        BodyMutator::new(
            body.blocks,
            locals,
            arg_count,
            debug_info,
            spread_arg,
            body.span,
            ghost_locals,
            ghost_blocks,
            statements,
        )
    }

    /// Index into the locals
    pub fn local(&self, idx: usize) -> &LocalDecl {
        if idx > self.locals.len() {
            &self.ghost_locals[idx - self.locals.len()]
        } else {
            &self.locals[idx]
        }
    }

    /// Create a new "ghost" local
    pub fn new_local(&mut self, ty: Ty, mutability: Mutability) -> Local {
        let span = self.span;
        let decl = LocalDecl { ty, span, mutability };
        let local = self.locals.len() + self.ghost_locals.len();
        self.ghost_locals.push(decl);
        local
    }

    /// Insert a call into the function body of the function stored at
    /// callee with the arguments in args.
    pub fn call(&mut self, callee: Local, args: Vec<Local>, local: Local) {
        let projection = Vec::new();
        let destination = Place { local, projection };
        let args = args
            .into_iter()
            .map(|v| Operand::Copy(Place { local: v, projection: vec![] }))
            .collect();
        let func = Operand::Copy(Place::from(callee));
        let unwind = UnwindAction::Terminate;
        let target = Some(self.next_block());
        let kind = TerminatorKind::Call { func, args, destination, target, unwind };
        let span = self.span;
        let terminator = Terminator { kind, span };
        let statements = std::mem::replace(&mut self.ghost_statements, Vec::new());
        self.ghost_blocks.push(BasicBlock { statements, terminator });
    }

    /// Finalize the prologue that initializes the variable data.
    pub fn finalize_prologue(&mut self) {
        let kind = TerminatorKind::Goto { target: self.blocks.len() - 1 };
        let span = self.span;
        let terminator = Terminator { kind, span };
        self.insert_bb(terminator);
    }

    /// Insert a ghost statement
    pub fn insert_statement(&mut self, stmt: Statement) {
        self.ghost_statements.push(stmt);
    }

    /// Get an index with which to iterate over the body
    pub fn new_index(&self) -> MutatorIndex {
        let len = self.blocks.len();
        let bb = std::cmp::max(len, 1) - 1;
        let idx = if len > 0 { std::cmp::max(self.blocks[bb].statements.len(), 1) - 1 } else { 0 };
        let span = self.span;
        MutatorIndex { bb, idx, span }
    }

    /// Decrement the index
    pub fn decrement(&self, index: &mut MutatorIndex) -> MutatorIndexStatus {
        let mut status = MutatorIndexStatus::Done;
        if index.idx > 0 || index.bb > 0 {
            status = MutatorIndexStatus::Remaining;
        }
        if index.idx > 0 {
            if index.idx < self.blocks[index.bb].statements.len() {
                index.span = self.blocks[index.bb].statements[index.idx].span;
            } else {
                index.span = self.blocks[index.bb].terminator.span;
            }
            index.idx -= 1;
        } else if index.bb > 0 {
            index.bb -= 1;
            index.span = self.blocks[index.bb].terminator.span;
            index.idx = self.blocks[index.bb].statements.len()
        }
        status
    }

    /// Inspect the index yielding the current statement or terminator
    pub fn inspect(&self, index: &MutatorIndex) -> Instruction {
        if index.idx >= self.blocks[index.bb].statements.len() {
            Instruction::Term(&self.blocks[index.bb].terminator)
        } else {
            Instruction::Stmt(&self.blocks[index.bb].statements[index.idx])
        }
    }

    /// Split at the given index, causing the current ghost code to be called
    /// and control flow to return from the ghost code to after the current index
    pub fn split(&mut self, index: &MutatorIndex) {
        let kind = TerminatorKind::Goto { target: self.blocks.len() + self.ghost_blocks.len() - 1 };
        let span = index.span;
        let term = Terminator { kind, span };
        let len = self.blocks[index.bb].statements.len();
        if index.idx < len {
            self.ghost_statements.extend(self.blocks[index.bb].statements.split_off(index.idx + 1));
        }
        let term = std::mem::replace(&mut self.blocks[index.bb].terminator, term);
        self.insert_bb(term);
    }

    /// Get the index of the next basic block
    pub fn next_block(&self) -> usize {
        self.blocks.len() + self.ghost_blocks.len() + 1
    }

    /// Insert a basic block with the given terminator
    pub fn insert_bb(&mut self, terminator: Terminator) {
        let statements = std::mem::replace(&mut self.ghost_statements, Vec::new());
        let execute_original_body = BasicBlock { statements, terminator };
        self.ghost_blocks.push(execute_original_body);
    }

    // Finalize the body mutator yielding a body
    pub fn finalize(self) -> Body {
        match self {
            BodyMutator {
                mut blocks,
                mut locals,
                arg_count,
                var_debug_info,
                spread_arg,
                span,
                ghost_locals,
                ghost_blocks,
                ghost_statements,
            } => {
                assert!(ghost_statements.len() == 0);
                blocks.extend(ghost_blocks.into_iter());
                locals.extend(ghost_locals.into_iter());
                Body::new(blocks, locals, arg_count, var_debug_info, spread_arg, span)
            }
        }
    }

    /// Get the span
    pub fn span(&self) -> Span {
        self.span
    }
}

/// Mutator index with which to iterate over the function body.
/// when idx = len(blocks[bb]), you are at the terminator, otherwise,
/// you are at the statement idx in the basic block blocks[bb].
#[derive(Debug)]
pub struct MutatorIndex {
    bb: BasicBlockIdx,
    idx: usize,
    span: Span,
}

/// Whether or not there is remaining code
#[derive(PartialEq, Eq)]
pub enum MutatorIndexStatus {
    Remaining,
    Done,
}

/// The instruction under inspection
pub enum Instruction<'a> {
    Stmt(&'a Statement),
    #[allow(unused)]
    Term(&'a Terminator),
}
