// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks
// compile-flags: -D warnings

struct Wrapped {
    pointer: *mut u8,
}

fn write(wrapper: Wrapped) {
    unsafe { *wrapper.pointer = 7 };
}

#[inline(never)]
fn indirect(f: fn(Wrapped), wrapper: Wrapped) {
    f(wrapper);
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
    indirect(write, Wrapped { pointer: &raw mut value });
    assert_eq!(value, 7);
}
