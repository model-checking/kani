// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module hosts the context used by Kani to convert MIR into Boogie.  See
//! the file level comments for more details.

mod boogie_ctx;
mod kani_intrinsic;
mod smt_builtins;

pub use boogie_ctx::BoogieCtx;
