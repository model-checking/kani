// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness check_stub --enable-unstable --enable-stubbing
//! Test that stub can solve glob cycles.

pub mod mod_a {
    pub use crate::mod_b::*;
    pub use crate::*;

    /// This method always fail.
    pub fn method_a() {
        noop();
        panic!();
    }
}

pub mod mod_b {
    pub use crate::mod_a::*;

    /// This harness replace `method_a` which always fail by `method_b` that should always succeed.
    #[kani::proof]
    #[kani::stub(mod_a::method_a, mod_b::noop)]
    pub fn check_stub() {
        method_a();
    }

    /// This method always succeed.
    pub fn noop() {}
}
