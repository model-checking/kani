// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains typesafe representations of CBMC's data structures

mod builtin;
mod expr;
mod gen_c_transformer;
mod identity_transformer;
mod location;
mod name_normalize_transformer;
mod stmt;
mod symbol;
mod symbol_table;
mod transformer;
mod typ;

pub use builtin::BuiltinFn;
pub use expr::{
    ArithmeticOverflowResult, BinaryOperand, Expr, ExprValue, SelfOperand, UnaryOperand,
};
pub use gen_c_transformer::GenCTransformer;
pub use identity_transformer::IdentityTransformer;
pub use location::Location;
pub use name_normalize_transformer::NameTransformer;
pub use stmt::{Stmt, StmtBody, SwitchCase};
pub use symbol::{Symbol, SymbolValues};
pub use symbol_table::SymbolTable;
pub use transformer::Transformer;
pub use typ::{CIntType, DatatypeComponent, Parameter, Type};
