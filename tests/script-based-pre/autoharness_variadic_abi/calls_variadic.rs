// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Asking `rustc_public` for the ABI of a non-C variadic used to abort the compilation, so a
//! harness that reaches one reported nothing at all: https://github.com/model-checking/kani/issues/4817

#[unsafe(naked)]
unsafe extern "sysv64" fn variadic_sysv64(_: ...) -> u32 {
    core::arch::naked_asm!("")
}

unsafe extern "sysv64" {
    fn foreign_variadic_sysv64(a: u32, _: ...) -> u32;
}

#[kani::proof]
fn check_defined() {
    unsafe {
        let _ = variadic_sysv64();
    }
}

#[kani::proof]
fn check_foreign() {
    unsafe {
        let _ = foreign_variadic_sysv64(3);
    }
}
