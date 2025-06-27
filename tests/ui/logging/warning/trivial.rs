// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --only-codegen
//! This test is to make sure we are correctly printing warnings from the kani-compiler.

pub fn asm() {
    unsafe {
        std::arch::asm!("NOP");
    }
}

fn is_true(b: bool) {
    assert!(b);
}

fn maybe_call<F: Fn() -> ()>(should_call: bool, f: F) {
    if should_call {
        f();
    }
}

// Duplicate proof harness attributes should produce a warning
#[kani::proof]
fn harness() {
    is_true(true);
    maybe_call(false, &asm);
}
