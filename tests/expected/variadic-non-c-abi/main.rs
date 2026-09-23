// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Kani cannot model a C-variadic function whose calling convention is not the C one, so calling
//! one reports an unsupported construct. Asking `rustc_public` for such a function's ABI used to
//! abort the compilation instead: https://github.com/model-checking/kani/issues/4817
//!
//! C-variadics themselves are supported, c.f. tests/kani/FunctionCall/Variadic.

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
