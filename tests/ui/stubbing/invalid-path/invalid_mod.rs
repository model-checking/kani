// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness invalid_stub -Z stubbing

pub mod mod_a {
    use crate::mod_b::noop;

    /// This method always fail.
    pub fn method_a() {
        noop();
        panic!();
    }
}

pub mod mod_b {
    pub use crate::mod_a::method_a;

    #[cfg_attr(kani, kani::proof)]
    #[cfg_attr(kani, kani::stub(crate::mod_a::method_a::invalid, noop))]
    pub fn invalid_stub() {
        method_a();
    }

    /// This method always succeed.
    pub fn noop() {}
}
