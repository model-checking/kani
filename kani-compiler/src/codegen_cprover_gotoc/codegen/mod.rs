// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module does that actual translation of MIR constructs to goto constructs.
//! Each subfile is named for the MIR construct it translates.

mod assert;
mod block;
mod foreign_function;
mod function;
mod intrinsic;
mod operand;
mod place;
mod rvalue;
mod span;
mod statement;
mod static_var;

// Visible for all codegen module.
mod ty_stable;
pub(super) mod typ;

pub use assert::PropertyClass;
pub use block::bb_label;
pub use typ::TypeExt;
