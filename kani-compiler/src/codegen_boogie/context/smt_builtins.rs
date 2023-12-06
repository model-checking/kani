// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use boogie_ast::boogie_program::{Function, Parameter, Type};

/// SMT bit-vector builtin operations, i.e. operations that SMT solvers (e.g.
/// Z3) understand
/// See https://smtlib.cs.uiowa.edu/logics-all.shtml for more details
#[derive(Debug, Clone, PartialEq, Eq, strum_macros::AsRefStr, strum_macros::EnumIter)]
pub(crate) enum SmtBvBuiltin {
    // Predicates:
    #[strum(serialize = "$BvUnsignedLessThan")]
    UnsignedLessThan,
    #[strum(serialize = "$BvSignedLessThan")]
    SignedLessThan,
    #[strum(serialize = "$BvUnsignedGreaterThan")]
    UnsignedGreaterThan,
    #[strum(serialize = "$BvSignedGreaterThan")]
    SignedGreaterThan,

    // Binary operators:
    #[strum(serialize = "$BvAdd")]
    Add,
    #[strum(serialize = "$BvOr")]
    Or,
    #[strum(serialize = "$BvAnd")]
    And,
    #[strum(serialize = "$BvShl")]
    Shl,
    #[strum(serialize = "$BvShr")]
    Shr,
}

impl SmtBvBuiltin {
    /// The name of the SMT function corresponding to this bit-vector operation
    pub fn smt_op_name(&self) -> &'static str {
        match self {
            SmtBvBuiltin::UnsignedLessThan => "bvult",
            SmtBvBuiltin::SignedLessThan => "bvslt",
            SmtBvBuiltin::UnsignedGreaterThan => "bvugt",
            SmtBvBuiltin::SignedGreaterThan => "bvsgt",
            SmtBvBuiltin::Add => "bvadd",
            SmtBvBuiltin::Or => "bvor",
            SmtBvBuiltin::And => "bvand",
            SmtBvBuiltin::Shl => "bvshl",
            SmtBvBuiltin::Shr => "bvlshr",
        }
    }

    /// Whether the builtin is a predicate (i.e. it returns a boolean)
    pub fn is_predicate(&self) -> bool {
        match self {
            SmtBvBuiltin::UnsignedLessThan
            | SmtBvBuiltin::SignedLessThan
            | SmtBvBuiltin::UnsignedGreaterThan
            | SmtBvBuiltin::SignedGreaterThan => true,
            SmtBvBuiltin::Or
            | SmtBvBuiltin::And
            | SmtBvBuiltin::Add
            | SmtBvBuiltin::Shl
            | SmtBvBuiltin::Shr => false,
        }
    }
}

/// Create a Boogie function for the given SMT bit-vector builtin
/// The function has no body, and is annotated with the SMT annotation
/// `:bvbuiltin "smt_name"` where `smt_name` is the SMT name of the bit-vector
/// builtin
pub(crate) fn smt_builtin_binop(
    bv_builtin: &SmtBvBuiltin,
    smt_name: &str,
    is_predicate: bool,
) -> Function {
    let tp_name = String::from("T");
    let tp = Type::parameter(tp_name.clone());
    Function::new(
        bv_builtin.as_ref().to_string(), // e.g. $BvOr
        vec![tp_name],
        vec![Parameter::new("lhs".into(), tp.clone()), Parameter::new("rhs".into(), tp.clone())],
        if is_predicate { Type::Bool } else { tp },
        None,
        vec![format!(":bvbuiltin \"{}\"", smt_name)],
    )
}
