// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains the representations of CBMC's daa structures.
//!
//! The encoding in this module directly maps to the one in CBMC, and thus one might find it clearer
//! in CBMC's documentation. All these representations precisely replicate ones in CBMC.
//!
//! In short, CBMC's AST has three levels:
//! 1. [SymbolTable] is the top level symbol table.
//! 2. [Symbol] is a symbol in the symbol table.
//! 3. [Irep] represents all trees (code, expression, metadata, etc).
//!
//! Each tree represented by [Irep] has three nodes:
//! 1. [id] for identity,
//! 2. [sub] for a (potentially empty) list of unnamed subtrees as [Irep],
//! 3. [named_sub] for a (potentially empty) map of named subtrees as [Irep].
//!
//! The function of a tree is usually (but not always) recognized by its [id], and thus the aggregation
//! of all recognized [id]s are represented by [IrepId]. [sub] usually contains operands and [named_sub]
//! usually contains other flags or metadata. For example, for a binary operation [a + b], the [id] of
//! this tree is ["plus"] denoting the tree being a plus operation. [sub] contains two [Irep]s
//! representing [a] and [b] respectively. [named_sub] contains other information include the type of
//! the expression, location information, and so on.
//!
//! Speical [id]s include:
//! 1. [Empty] and [Nil] behaves like [null].

mod env;
pub mod goto_program;
pub mod irep;
mod machine_model;
pub mod utils;
pub use machine_model::{MachineModel, RoundingMode};
