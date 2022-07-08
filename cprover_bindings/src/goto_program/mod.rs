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
pub mod symtab_transformer;
mod typ;

pub use builtin::BuiltinFn;
pub use contract::{Contract, Spec};
pub use expr::{
    ArithmeticOverflowResult, BinaryOperand, Expr, ExprValue, SelfOperand, UnaryOperand,
};
pub use location::Location;
pub use stmt::{Stmt, StmtBody, SwitchCase};
pub use symbol::{Symbol, SymbolValues};
pub use symbol_table::SymbolTable;
pub use typ::{CIntType, DatatypeComponent, Parameter, Type};
