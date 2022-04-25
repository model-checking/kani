// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains the structures used for symbol table transformations.

mod gen_c_transformer;
mod identity_transformer;
mod passes;
mod transformer;

pub use passes::do_passes;
use transformer::Transformer;
