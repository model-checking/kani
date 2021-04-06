// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module is am implementation of the `Irep` serilization format for goto programs.
//!
//! It is generally not expected that you will create these directly.
//! Instead, this module is a bridge between the typesafe datastructures in the `goto_program` module, and the un-typed `irep` represenation used internally by CBMC.
//! You almost certainly want to create typesafe `goto_program` structures, and use the `to_irep` trait from this module to create canonical ireps from them.
//! This module also supports getting typesafe `goto_program` structures from an irep, and hence can serve as the intermediate phase in a `goto` to `goto` translator.
//!
//! Internally, this module uses the naïve representation of an irep as a node with concrete named and unnamed subtrees.
//! This representation does not take advantage of the sharing features available for ireps in CBMC to reduce memory usage.
//!
//! TODO: Complete the from-irep trait for remaining data types
//! TODO: Parser for json symbol tables into the internal irep format
//! TODO: Investigate memory usage, and consider using sharing to reduce memory usage

mod from_irep;
mod irep;
mod irep_id;
mod symbol;
mod symbol_table;
mod to_irep;
mod to_json;

pub use irep::Irep;
pub use irep_id::IrepId;
pub use symbol::Symbol;
pub use symbol_table::SymbolTable;
pub use to_irep::ToIrep;
