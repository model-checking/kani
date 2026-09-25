// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! These harnesses exist only when Kani compiles for a riscv64 target, so they are verified only
//! with `--target riscv64gc-unknown-linux-gnu`.

#[cfg(target_arch = "riscv64")]
mod riscv64 {
    /// Facts that hold for `riscv64gc-unknown-linux-gnu` and not for every host Kani runs on:
    /// `c_char` is unsigned there but signed on x86_64, and the LP64D ABI needs the `d` feature.
    #[kani::proof]
    fn target_is_riscv64gc() {
        assert!(core::ffi::c_char::MIN == 0);
        assert!(cfg!(target_feature = "d"));
        assert!(core::mem::size_of::<usize>() == 8);
    }

    /// A failing check, so the test also shows that CBMC checked the riscv64 program.
    #[kani::proof]
    fn overflow_is_found() {
        let x: u32 = kani::any();
        let _ = x + 1;
    }
}
