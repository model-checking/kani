// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains typesafe representations of CBMC's data structures

mod builtin;
mod expr;
mod identity_transformer;
mod location;
mod stmt;
mod symbol;
mod symbol_table;
mod transformer;
mod typ;

pub use builtin::BuiltinFn;
pub use expr::{
    ArithmeticOverflowResult, BinaryOperand, Expr, ExprValue, SelfOperand, UnaryOperand,
};
pub use identity_transformer::IdentityTransformer;
pub use location::Location;
pub use stmt::{Stmt, StmtBody, SwitchCase};
pub use symbol::{Symbol, SymbolValues};
pub use symbol_table::SymbolTable;
pub use transformer::Transformer;
pub use typ::{CIntType, DatatypeComponent, Parameter, Type};
