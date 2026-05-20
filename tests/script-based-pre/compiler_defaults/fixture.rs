// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A `#[cfg(kani)]`-gated harness. If kani-compiler sets `--cfg=kani` as a
//! default, the harness compiles; if not, the module is invisible and 0
//! harnesses appear in the metadata — a vacuous "verification" with nothing
//! verified.

#[cfg(kani)]
mod verify {
    #[kani::proof]
    fn check_with_defaults() {
        let x: u8 = kani::any();
        assert_eq!(x.wrapping_mul(1), x);
    }
}
