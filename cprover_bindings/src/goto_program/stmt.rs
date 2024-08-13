// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use self::StmtBody::*;
use super::{BuiltinFn, Expr, Location};
use crate::{InternString, InternedString};
use std::fmt::Debug;

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
    /// `assert(cond)`
    Assert {
        cond: Expr,
        property_class: InternedString,
        msg: InternedString,
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
    /// End-of-life of a local variable
    Dead(Expr),
    /// `lhs.typ lhs = value;` or `lhs.typ lhs;`
    Decl {
        lhs: Expr, // SymbolExpr
        value: Option<Expr>,
    },
    /// Marks the target place as uninitialized.
    Deinit(Expr),
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
    Goto {
        dest: InternedString,
        // The loop invariants annotated to the goto, which can be
        // applied as loop contracts in CBMC if it is a backward goto.
        loop_invariants: Option<Expr>,
    },
    /// `if (i) { t } else { e }`
    Ifthenelse {
        i: Expr,
        t: Stmt,
        e: Option<Stmt>,
    },
    /// `label: body;`
    Label {
        label: InternedString,
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

    // If self has a body of type `Block(stmts)`, return `stmts`; otherwise, None
    pub fn get_stmts(&self) -> Option<&Vec<Stmt>> {
        match self.body() {
            Block(stmts) => Some(stmts),
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
        assert_eq!(
            lhs.typ(),
            rhs.typ(),
            "Error: assign statement with unequal types lhs {:?} rhs {:?}",
            lhs.typ(),
            rhs.typ()
        );
        stmt!(Assign { lhs, rhs }, loc)
    }

    /// `assert(cond, property_class, comment);`
    pub fn assert(cond: Expr, property_name: &str, message: &str, loc: Location) -> Self {
        assert!(cond.typ().is_bool());
        assert!(!property_name.is_empty() && !message.is_empty());

        // Create a Property Location Variant from any given Location type
        let loc_with_property =
            Location::create_location_with_property(message, property_name, loc);

        // Chose InternedString to separate out codegen from the cprover_bindings logic
        let property_class = property_name.intern();
        let msg = message.into();

        stmt!(Assert { cond, property_class, msg }, loc_with_property)
    }

    pub fn assert_false(property_name: &str, message: &str, loc: Location) -> Self {
        Stmt::assert(Expr::bool_false(), property_name, message, loc)
    }

    /// `__CPROVER_assume(cond);`
    pub fn assume(cond: Expr, loc: Location) -> Self {
        assert!(cond.typ().is_bool(), "Assume expected bool, got {cond:?}");
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

    /// `__CPROVER_cover(cond);`
    /// This has the same semantics as `__CPROVER_assert(!cond)`, but the
    /// difference is in how CBMC reports their results: instead of
    /// SUCCESS/FAILURE, it uses SATISFIED/FAILED
    pub fn cover(cond: Expr, loc: Location) -> Self {
        assert!(cond.typ().is_bool());
        BuiltinFn::CProverCover.call(vec![cond], loc).as_stmt(loc)
    }

    /// Local variable goes out of scope
    pub fn dead(symbol: Expr, loc: Location) -> Self {
        stmt!(Dead(symbol), loc)
    }

    /// `lhs.typ lhs = value;` or `lhs.typ lhs;`
    pub fn decl(lhs: Expr, value: Option<Expr>, loc: Location) -> Self {
        assert!(lhs.is_symbol());
        assert!(value.iter().all(|x| lhs.typ() == x.typ()));
        stmt!(Decl { lhs, value }, loc)
    }

    /// `Deinit(place)`, see `StmtBody::Deinit`.
    pub fn deinit(place: Expr, loc: Location) -> Self {
        stmt!(Deinit(place), loc)
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
            "Function call does not type check:\nfunc: {function:?}\nargs: {arguments:?}"
        );
        if let Some(lhs) = &lhs {
            assert_eq!(lhs.typ(), function.typ().return_type().unwrap())
        }
        stmt!(FunctionCall { lhs, function, arguments }, loc)
    }

    /// `goto dest;`
    pub fn goto<T: Into<InternedString>>(dest: T, loc: Location) -> Self {
        let dest = dest.into();
        assert!(!dest.is_empty());
        stmt!(Goto { dest, loop_invariants: None }, loc)
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
    pub fn with_label<T: Into<InternedString>>(self, label: T) -> Self {
        let label = label.into();
        assert!(!label.is_empty());
        stmt!(Label { label, body: self }, *self.location())
    }

    /// `goto dest;` with loop invariant
    pub fn with_loop_contracts(self, inv: Expr) -> Self {
        if let Goto { dest, loop_invariants } = self.body() {
            assert!(loop_invariants.is_none());
            stmt!(Goto { dest: *dest, loop_invariants: Some(inv) }, *self.location())
        } else {
            unreachable!("Loop contracts should be annotated only to goto stmt")
        }
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
