// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains the representations of CBMC's daa structures.
//!
//! The encoding in this module directly maps to the one in CBMC, and thus one might find it clearer
//! in CBMC's documentation. All these representations precisely replicate ones in CBMC.
//!
//! In short, CBMC's AST has three levels:
//! 1. [irep::SymbolTable] is the top level symbol table.
//! 2. [irep::Symbol] is a symbol in the symbol table.
//! 3. [irep::Irep] represents all trees (code, expression, metadata, etc).
//!
//! Each tree represented by [irep::Irep] has three nodes:
//! 1. [irep::Irep::id] for identity,
//! 2. [irep::Irep::sub] for a (potentially empty) list of unnamed subtrees as [irep::Irep],
//! 3. [irep::Irep::named_sub] for a (potentially empty) map of named subtrees as [irep::Irep].
//!
//! The function of a tree is usually (but not always) recognized by
//! its [irep::Irep::id], and thus the aggregation of all recognized
//! [irep::Irep::id]s are represented by [irep::IrepId]. [irep::Irep::sub] usually
//! contains operands and [irep::Irep::named_sub] usually contains
//! other flags or metadata. For example, for a binary operation [a +
//! b], the [irep::Irep::id] of this tree is ["plus"] denoting the
//! tree being a plus operation. [irep::Irep::sub] contains two [irep::Irep]s
//! representing \[a\] and \[b\] respectively. [irep::Irep::named_sub]
//! contains other information include the type of the expression,
//! location information, and so on.
//!
//! Speical [irep::Irep::id]s include:
//! 1. [irep::IrepId::Empty] and [irep::IrepId::Nil] behaves like \[null\].

#![feature(f128)]
#![feature(f16)]

mod env;
pub mod goto_program;
pub mod irep;
mod machine_model;
pub mod utils;
pub use irep::serialize;
pub use machine_model::{MachineModel, RoundingMode};
mod cbmc_string;
pub use cbmc_string::{InternString, InternStringOption, InternedString};
