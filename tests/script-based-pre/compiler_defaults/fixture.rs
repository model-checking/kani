// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! `#[cfg(kani)]`-gated harnesses. If kani-compiler sets `--cfg=kani` as a
//! default, the harnesses compile; if not, the module is invisible and 0
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

    /// rustc derives `cfg(panic = "abort")` from the RESOLVED panic
    /// strategy, so this harness is in the metadata count only if
    /// `-Cpanic=abort` won the session — a backstop for the conflict case
    /// alongside kani-compiler's own abort-strategy gate.
    #[cfg(panic = "abort")]
    #[kani::proof]
    fn check_panic_abort_wins() {
        let x: u8 = kani::any();
        assert_eq!(x.wrapping_mul(1), x);
    }
}

#[cfg(not_a_kani_cfg)]
mod control {}
