// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks
// compile-flags: -D warnings

static mut ESCAPED: *mut u8 = std::ptr::null_mut();

fn write() {
    unsafe { *ESCAPED = 7 };
}

#[inline(never)]
fn indirect(f: fn()) {
    // The callee accesses memory through a global pointer.
    f();
}

#[kani::proof]
fn check() {
    let mut value = 0u8;
    unsafe { ESCAPED = &raw mut value };
    indirect(write);
    assert_eq!(value, 7);
}
