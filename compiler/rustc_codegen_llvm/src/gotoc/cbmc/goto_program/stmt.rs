// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use self::StmtBody::*;
use super::{BuiltinFn, Expr, Location};
use std::fmt::Debug;
use tracing::debug;

///////////////////////////////////////////////////////////////////////////////////////////////
/// Datatypes
///////////////////////////////////////////////////////////////////////////////////////////////

/// An `Stmt` represents a statement type: i.e. a computation that does not return a value.
/// Every statement has a type and a location (which may be `None`).
///
/// The fields of `Stmt` are kept private, and there are no getters that return mutable references.
/// This means that the only way to create and update `Stmt`s is using the constructors and setters.
/// The constructors ensure that statements are well formed.
///
/// In general, statements are constructed in a "function-call" style:
///     `while (c) {stmt1, stmt2}`
///      would translate to `Stmt::while_loop(c, vec![stmt1, stmt2], loc)`
/// Statements can also be created using the converters in the `Expr` module.
///
/// TODO:
/// The CBMC irep resentation uses sharing to reduce the in-memory size of expressions.
/// This is not currently implemented for these expressions, but would be possible given a factory.
#[derive(Debug, Clone)]
pub struct Stmt {
    body: Box<StmtBody>,
    location: Location,
}
/// The different kinds of bodies a statement can have.
/// The names are chosen to map directly onto the IrepID used by CBMC.
/// Each statement is described by reference to the corresponding C code that would generate it.
#[derive(Debug, Clone)]
pub enum StmtBody {
    /// `lhs = rhs;`
    Assign {
        lhs: Expr,
        rhs: Expr,
    },
    /// `__CPROVER_assume(cond);`
    Assume {
        cond: Expr,
    },
    /// { ATOMIC_BEGIN stmt1; stmt2; ... ATOMIC_END }
    AtomicBlock(Vec<Stmt>),
    /// `{ stmt1; stmt2; ... }`
    Block(Vec<Stmt>),
    /// `break;`
    Break,
    /// `continue;`
    Continue,
    /// `lhs.typ lhs = value;` or `lhs.typ lhs;`
    Decl {
        lhs: Expr, // SymbolExpr
        value: Option<Expr>,
    },
    /// `e;`
    Expression(Expr),
    // `for (init; cond; update) {body}`
    For {
        init: Stmt,
        cond: Expr,
        update: Stmt,
        body: Stmt,
    },
    /// `lhs = function(arguments);` or `function(arguments);`
    FunctionCall {
        lhs: Option<Expr>,
        function: Expr,
        arguments: Vec<Expr>,
    },
    /// `goto dest;`
    Goto(String),
    /// `if (i) { t } else { e }`
    Ifthenelse {
        i: Expr,
        t: Stmt,
        e: Option<Stmt>,
    },
    /// `label: body;`
    Label {
        label: String,
        body: Stmt,
    },
    /// `return e;` or `return;`
    Return(Option<Expr>),
    /// `;`
    Skip,
    /// `switch (control) { case1.case: cast1.body; case2.case: case2.body; ... }`
    Switch {
        control: Expr,
        cases: Vec<SwitchCase>,
        default: Option<Stmt>,
    },
    /// `while (cond) { body }`
    While {
        cond: Expr,
        body: Stmt,
    },
}

#[derive(Debug, Clone)]
pub struct SwitchCase {
    case: Expr,
    body: Stmt,
}

///////////////////////////////////////////////////////////////////////////////////////////////
/// Implementations
///////////////////////////////////////////////////////////////////////////////////////////////

/// Getters
impl Stmt {
    pub fn body(&self) -> &StmtBody {
        &self.body
    }

    pub fn location(&self) -> &Location {
        &self.location
    }

    /// If self has a body of type `Expression`, return its body; otherwise, None
    pub fn get_expression(&self) -> Option<&Expr> {
        match self.body() {
            Expression(e) => Some(e),
            _ => None,
        }
    }
}

/// Fluent builders
impl Stmt {
    /// Chained call to allow chained pattern
    pub fn with_location(mut self, loc: Location) -> Self {
        self.location = loc;
        self
    }
}

macro_rules! stmt {
    ( $body:expr, $loc:expr) => {{
        let location = $loc;
        let body = Box::new($body);
        Stmt { body, location }
    }};
}

/// Constructors
impl Stmt {
    /// `lhs = rhs;`
    pub fn assign(lhs: Expr, rhs: Expr, loc: Location) -> Self {
        //Temporarily work around https://github.com/model-checking/rmc/issues/95
        //by disabling the assert and soundly assigning nondet
        //assert_eq!(lhs.typ(), rhs.typ());
        if lhs.typ() != rhs.typ() {
            debug!(
                "WARNING: assign statement with unequal types lhs {:?} rhs {:?}",
                lhs.typ(),
                rhs.typ()
            );
            let assert_stmt = Stmt::assert_false(
                &format!(
                    "Reached assignment statement with unequal types {:?} {:?}",
                    lhs.typ(),
                    rhs.typ()
                ),
                loc.clone(),
            );
            let nondet_value = lhs.typ().nondet();
            let nondet_assign_stmt = stmt!(Assign { lhs, rhs: nondet_value }, loc.clone());
            return Stmt::block(vec![assert_stmt, nondet_assign_stmt], loc);
        }
        stmt!(Assign { lhs, rhs }, loc)
    }

    /// `__CPROVER_assert(cond, msg);`
    pub fn assert(cond: Expr, msg: &str, loc: Location) -> Self {
        assert!(cond.typ().is_bool());
        BuiltinFn::CProverAssert
            .call(vec![cond, Expr::string_constant(msg)], loc.clone())
            .as_stmt(loc)
    }

    pub fn assert_false(msg: &str, loc: Location) -> Self {
        Stmt::assert(Expr::bool_false(), msg, loc)
    }

    /// A __CPROVER_assert to sanity check expected components of code
    /// generation. If users see these assertions fail, something in the
    /// translation to Gotoc has gone wrong, and we want them to file an issue.
    pub fn assert_sanity_check(expect_true: Expr, message: &str, url: &str, loc: Location) -> Stmt {
        let assert_msg =
            format!("Code generation sanity check: {}. Please report failures:\n{}", message, url);

        Stmt::block(
            vec![
                // Assert our expected true expression.
                Stmt::assert(expect_true.clone(), &assert_msg, loc.clone()),
                // If expect_true is false, assume false to block any further
                // exploration of this path.
                Stmt::assume(expect_true, loc.clone()),
            ],
            loc,
        )
    }

    /// `__CPROVER_assume(cond);`
    pub fn assume(cond: Expr, loc: Location) -> Self {
        assert!(cond.typ().is_bool(), "Assume expected bool, got {:?}", cond);
        stmt!(Assume { cond }, loc)
    }

    /// { ATOMIC_BEGIN stmt1; stmt2; ... ATOMIC_END }
    pub fn atomic_block(stmts: Vec<Stmt>, loc: Location) -> Self {
        stmt!(AtomicBlock(stmts), loc)
    }

    /// `{ stmt1; stmt2; ... }`
    pub fn block(stmts: Vec<Stmt>, loc: Location) -> Self {
        stmt!(Block(stmts), loc)
    }

    /// `break;`
    pub fn break_stmt(loc: Location) -> Self {
        stmt!(Break, loc)
    }

    /// `continue;`
    pub fn continue_stmt(loc: Location) -> Self {
        stmt!(Continue, loc)
    }

    /// `lhs.typ lhs = value;` or `lhs.typ lhs;`
    pub fn decl(lhs: Expr, value: Option<Expr>, loc: Location) -> Self {
        assert!(lhs.is_symbol());
        assert!(value.iter().all(|x| lhs.typ() == x.typ()));
        stmt!(Decl { lhs, value }, loc)
    }

    /// `e;`
    pub fn code_expression(e: Expr, loc: Location) -> Self {
        stmt!(Expression(e), loc)
    }

    // `for (init; cond; update) {body}`
    pub fn for_loop(init: Stmt, cond: Expr, update: Stmt, body: Stmt, loc: Location) -> Self {
        assert!(cond.typ().is_bool());
        stmt!(For { init, cond, update, body }, loc)
    }

    /// `lhs = function(arguments);` or `function(arguments);`
    pub fn function_call(
        lhs: Option<Expr>,
        function: Expr,
        arguments: Vec<Expr>,
        loc: Location,
    ) -> Self {
        assert!(
            Expr::typecheck_call(&function, &arguments),
            "Function call does not type check:\nfunc: {:?}\nargs: {:?}",
            function,
            arguments
        );
        if let Some(lhs) = &lhs {
            assert_eq!(lhs.typ(), function.typ().return_type().unwrap())
        }
        stmt!(FunctionCall { lhs, function, arguments }, loc)
    }

    /// `goto dest;`
    pub fn goto(dest: String, loc: Location) -> Self {
        assert!(!dest.is_empty());
        stmt!(Goto(dest), loc)
    }

    /// `if (i) { t } else { e }` or `if (i) { t }`
    pub fn if_then_else(i: Expr, t: Stmt, e: Option<Stmt>, loc: Location) -> Self {
        assert!(i.typ().is_bool());
        stmt!(Ifthenelse { i, t, e }, loc)
    }

    /// `return e;` or `return;`
    pub fn ret(e: Option<Expr>, loc: Location) -> Self {
        stmt!(Return(e), loc)
    }

    /// `;`
    pub fn skip(loc: Location) -> Self {
        stmt!(Skip, loc)
    }

    /// `switch (control) { case1.case: cast1.body; case2.case: case2.body; ... }`
    pub fn switch(
        control: Expr,
        cases: Vec<SwitchCase>,
        default: Option<Stmt>,
        loc: Location,
    ) -> Self {
        assert!(cases.iter().all(|x| x.case().typ() == control.typ()));
        stmt!(Switch { control, cases, default }, loc)
    }

    /// `while (cond) { body }`
    pub fn while_loop(cond: Expr, body: Stmt, loc: Location) -> Self {
        assert!(cond.typ().is_bool());
        stmt!(While { cond, body }, loc)
    }

    /// `label: self;`
    pub fn with_label(self, label: String) -> Self {
        assert!(!label.is_empty());
        stmt!(Label { label, body: self }, self.location().clone())
    }
}

/// Predicates
impl Stmt {
    pub fn is_expression(&self) -> bool {
        match self.body() {
            StmtBody::Expression(_) => true,
            _ => false,
        }
    }
}

/// Setters
impl StmtBody {
    #[deprecated(
        note = "Instead, collect the statements, and create an immutable block at the end."
    )]
    pub fn insert_sub(&mut self, s: Stmt) {
        match self {
            StmtBody::Block(stmts) => stmts.push(s),
            _ => unreachable!("Can't push into something that's not a block"),
        }
    }
}

/// Constructors
impl SwitchCase {
    /// case : body;
    // TODO figure out if these have a `break` or implement fallthrough
    pub fn new(case: Expr, body: Stmt) -> Self {
        SwitchCase { case, body }
    }
}

/// Getters
impl SwitchCase {
    pub fn case(&self) -> &Expr {
        &self.case
    }

    pub fn body(&self) -> &Stmt {
        &self.body
    }
}
