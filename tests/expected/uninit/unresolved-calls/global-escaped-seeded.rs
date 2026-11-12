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
    // There is no pointer argument or return value to gate instrumentation on.
    f();
}

// A copy intrinsic creates a delayed-UB analysis target without introducing UB.
#[inline(never)]
fn seed() {
    let source = 7u8;
    let mut destination = 0u8;
    unsafe { std::ptr::copy_nonoverlapping(&source, &mut destination, 1) };
    assert_eq!(destination, source);
}

#[kani::proof]
fn check() {
    seed();
    let mut value = 0u8;
    unsafe { ESCAPED = &raw mut value };
    indirect(write);
    assert_eq!(value, 7);
}
