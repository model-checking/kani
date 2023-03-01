// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness check_stub --enable-unstable --enable-stubbing
//! Test that stub can solve glob cycles even when the path expands the cycle.

pub mod mod_a {
    pub use crate::mod_b::*;
    pub use crate::*;

    /// This method always fails.
    pub fn method_a() {
        mod_a::mod_b::mod_a::mod_b::noop();
        panic!();
    }
}

pub mod mod_b {
    pub use crate::mod_a::*;
    pub use crate::*;

    /// This harness replaces `method_a` (always fails), by `method_b` (always succeeds).
    #[kani::proof]
    #[kani::stub(mod_a::mod_b::mod_a::method_a, mod_b::noop)]
    pub fn check_stub() {
        method_a();
    }

    /// This method always succeeds.
    pub fn noop() {}
}
