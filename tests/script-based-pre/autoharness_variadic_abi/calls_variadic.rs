// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Asking `rustc_public` for the ABI of a non-C variadic used to abort the compilation, so a
//! harness that reaches one reported nothing at all: https://github.com/model-checking/kani/issues/4817

#[unsafe(naked)]
unsafe extern "sysv64" fn defined_variadic(_: u32, _: ...) -> u32 {
    core::arch::naked_asm!("")
}

unsafe extern "sysv64" {
    fn foreign_variadic_sysv64(a: u32, _: ...) -> u32;
}

#[kani::proof]
fn check_defined() {
    unsafe {
        let _ = defined_variadic(3);
    }
}

#[kani::proof]
fn check_foreign() {
    unsafe {
        let _ = foreign_variadic_sysv64(3);
    }
}

// Reaching one through a function pointer goes down a different codegen path, which asked
// `rustc_public` for the pointer's ABI and hit the same assertion.
#[kani::proof]
fn check_call_through_pointer() {
    let p: unsafe extern "sysv64" fn(u32, ...) -> u32 = defined_variadic;
    unsafe {
        let _ = p(3);
    }
}

// Taking the address without calling builds an FFI shim for the foreign declaration, which is
// where the calling convention used to be read off the unavailable ABI. Nothing unsupported is
// reached here, so this harness verifies.
#[kani::proof]
fn check_foreign_address_taken() {
    let p: unsafe extern "sysv64" fn(u32, ...) -> u32 = foreign_variadic_sysv64;
    assert!(!(p as *const ()).is_null());
}
