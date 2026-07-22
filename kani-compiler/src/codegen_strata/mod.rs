// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Strata IR codegen backend for Kani
//!
//! This module provides a codegen backend that translates Rust MIR to Strata Core dialect.

pub mod compiler_interface;
mod mir_to_strata;
mod strata_builder;

pub use compiler_interface::StrataCodegenBackend;
