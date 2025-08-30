// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The purpose of this crate is to allow kani to selectively override
//! definitions from the standard library.  Definitions provided below would
//! override the standard library versions.

// See discussion in
// https://rust-lang.zulipchat.com/#narrow/stream/182449-t-compiler.2Fhelp/topic/.E2.9C.94.20Globally.20override.20an.20std.20macro/near/268873354
// for more details.

// re-export all std symbols
pub use std::*;

#[cfg(not(feature = "concrete_playback"))]
// Override process calls with stubs.
pub mod process;

#[macro_use]
mod macros;
