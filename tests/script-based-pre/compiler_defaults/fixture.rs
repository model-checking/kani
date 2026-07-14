// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A `#[cfg(kani)]`-gated harness. If kani-compiler sets `--cfg=kani` as a
//! default, the harness compiles; if not, the module is invisible and 0
//! harnesses appear in the metadata — a vacuous "verification" with nothing
//! verified.
//!
//! `control` is a negative control for the `--check-cfg=cfg(kani)` default:
//! its cfg is deliberately undeclared, so the unexpected_cfgs warning it
//! draws proves cfg checking is active, while `#[cfg(kani)]` drawing no
//! warning proves `kani` is the name the default registered.

#[cfg(kani)]
mod verify {
    #[kani::proof]
    fn check_with_defaults() {
        let x: u8 = kani::any();
        assert_eq!(x.wrapping_mul(1), x);
    }
}

#[cfg(not_a_kani_cfg)]
mod control {}
