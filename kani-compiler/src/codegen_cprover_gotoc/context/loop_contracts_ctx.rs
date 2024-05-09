use crate::codegen_cprover_gotoc::codegen::bb_label;
use cbmc::goto_program::{CIntType, Expr, Stmt, StmtBody, Type};
use stable_mir::mir::BasicBlockIdx;
use std::collections::HashSet;

pub struct LoopContractsCtx {
    /// the GOTO block compiled from the corresponding loop invariants
    invariants_block: Vec<Stmt>,
    /// Which codegen state
    stage: LoopContractsStage,
    /// If enable loop contracts
    loop_contracts_enabled: bool,
    /// Seen basic block indexes. Used to decide if a jump is backward
    seen_bbidx: HashSet<BasicBlockIdx>,
    /// Current unused bbidx label
    current_bbidx_label: Option<String>,
    /// The lhs of evaluation of the loop invariant
    loop_invariant_lhs: Option<Stmt>,
}

/// We define two states:
/// 1. loop invariants block
///     In this state, we push all codegen stmts into the invariant block.
///     We enter this state when codegen for `KaniLoopInvariantBegin`.
///     We exit this state when codegen for `KaniLoopInvariantEnd`.
/// 2. loop latch block
///     In this state, we codegen a statement expression from the
///     invariant_block annotate the statement expression to the named sub
///     of the next backward jumping we codegen.
///     We enter this state when codegen for `KaniLoopInvariantEnd`.
///     We exit this state when codegen for the first backward jumping.
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
enum LoopContractsStage {
    /// Codegen for user code as usual
    UserCode,
    /// Codegen for loop invariants
    InvariantBlock,
    /// Codegen for loop latch node
    FindingLatchNode,
}

/// Constructor
impl LoopContractsCtx {
    pub fn new(loop_contracts_enabled: bool) -> Self {
        Self {
            invariants_block: Vec::new(),
            stage: LoopContractsStage::UserCode,
            loop_contracts_enabled: loop_contracts_enabled,
            seen_bbidx: HashSet::new(),
            current_bbidx_label: None,
            loop_invariant_lhs: None,
        }
    }
}

/// Getters
impl LoopContractsCtx {
    pub fn loop_contracts_enabled(&self) -> bool {
        self.loop_contracts_enabled
    }

    /// decide if a GOTO with `target` is backward jump
    pub fn is_loop_latch(&self, target: &BasicBlockIdx) -> bool {
        self.stage == LoopContractsStage::FindingLatchNode && self.seen_bbidx.contains(target)
    }
}

/// Setters
impl LoopContractsCtx {
    /// Returns the current block as a statement expression.
    /// Exit loop latch block.
    pub fn extract_block(&mut self) -> Expr {
        assert!(self.loop_invariant_lhs.is_some());
        self.stage = LoopContractsStage::UserCode;
        self.invariants_block.push(self.loop_invariant_lhs.as_ref().unwrap().clone());

        // The first statement is the GOTO in the rhs of __kani_loop_invariant_begin()
        // Ignore it
        self.invariants_block.remove(0);

        Expr::statement_expression(
            std::mem::take(&mut self.invariants_block),
            Type::CInteger(CIntType::Bool),
        )
        .cast_to(Type::bool())
    }

    /// Push the `s` onto the block if it is in the loop invariant block
    /// and return `skip`. Otherwise, do nothing and return `s`.
    pub fn push_onto_block(&mut self, s: Stmt) -> Stmt {
        if self.stage == LoopContractsStage::InvariantBlock {
            // Attach the lable to the first Stmt in that block and reset it.
            let to_push = if self.current_bbidx_label.is_none() {
                s.clone()
            } else {
                s.clone().with_label(self.current_bbidx_label.clone().unwrap())
            };
            self.current_bbidx_label = None;

            match s.body() {
                StmtBody::Assign { lhs, rhs: _ } => {
                    let lhs_stmt = lhs.clone().as_stmt(*s.location());
                    self.loop_invariant_lhs = Some(lhs_stmt.clone());
                    self.invariants_block.push(to_push);
                }
                _ => {
                    self.invariants_block.push(to_push);
                }
            };
            Stmt::skip(*s.location())
        } else {
            s
        }
    }

    pub fn enter_loop_invariant_block(&mut self) {
        assert!(self.invariants_block.is_empty());
        self.stage = LoopContractsStage::InvariantBlock;
    }

    pub fn exit_loop_invariant_block(&mut self) {
        self.stage = LoopContractsStage::FindingLatchNode;
    }

    /// Enter a new function, reset the seen_bbidx set
    pub fn enter_new_function(&mut self) {
        self.seen_bbidx = HashSet::new()
    }

    pub fn add_new_seen_bbidx(&mut self, bbidx: BasicBlockIdx) {
        self.seen_bbidx.insert(bbidx);
        self.current_bbidx_label = Some(bb_label(bbidx));
    }
}
