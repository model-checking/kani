// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains typesafe representations of CBMC's data structures

// There are a fair number of constructs in this module that are better maintained as
// explicit pattern matching versus using the `matches!` macro.
#![allow(clippy::match_like_matches_macro)]

mod builtin;
mod contract;
mod expr;
mod location;
mod stmt;
mod symbol;
mod symbol_table;
mod typ;

pub use builtin::BuiltinFn;
pub use contract::{Contract, ContractValue, Spec};
pub use expr::{
    arithmetic_overflow_result_type, ArithmeticOverflowResult, BinaryOperator, Expr, ExprValue,
    SelfOperator, UnaryOperator, ARITH_OVERFLOW_OVERFLOWED_FIELD, ARITH_OVERFLOW_RESULT_FIELD,
};
pub use location::Location;
pub use stmt::{Stmt, StmtBody, SwitchCase};
pub use symbol::{Symbol, SymbolValues};
pub use symbol_table::SymbolTable;
pub use typ::{CIntType, DatatypeComponent, Parameter, Type};
