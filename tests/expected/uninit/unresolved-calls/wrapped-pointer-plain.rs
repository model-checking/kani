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

#[kani::proof]
fn check() {
    let mut value = 0u8;
    indirect(write, Wrapped { pointer: &raw mut value });
    assert_eq!(value, 7);
}
