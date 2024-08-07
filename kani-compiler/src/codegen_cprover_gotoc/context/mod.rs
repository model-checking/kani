// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module carries the context used by Kani to convert MIR into goto.
//! Kani can be thought of as a translator from an MIR context to a goto context.
//! This struct `GotocCtx<'tcx>` defined in this module, tracks both views of information.
//! See the file level comments for more details.

mod current_fn;
mod goto_ctx;
mod loop_contracts_ctx;
mod vtable_ctx;

pub use goto_ctx::GotocCtx;
pub use vtable_ctx::VtableCtx;
