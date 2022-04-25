// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains typesafe representations of CBMC's data structures

mod builtin;
mod expr;
mod location;
mod stmt;
mod symbol;
mod symbol_table;
pub mod symtab_transformer;
mod typ;

pub use builtin::BuiltinFn;
pub use expr::{
    ArithmeticOverflowResult, BinaryOperand, Expr, ExprValue, SelfOperand, UnaryOperand,
};
pub use location::Location;
pub use stmt::{Stmt, StmtBody, SwitchCase};
pub use symbol::{Symbol, SymbolValues};
pub use symbol_table::SymbolTable;
pub use typ::{CIntType, DatatypeComponent, Parameter, Type};
