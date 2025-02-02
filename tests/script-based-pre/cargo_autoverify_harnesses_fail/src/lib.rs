// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zautomatic-harnesses

// Test the bodies of the automatically generated harnesses:
// do they catch the same failures as manual ones?
// Note that this also indirectly tests that the automatic harness
// subcommand also runs the manual harnesses in the crate.

fn oob_safe_array_access(idx: usize) {
    let v = vec![1, 2, 3];
    v[idx];
}

fn oob_unsafe_array_access(i: usize) -> u32 {
    let a: &[u32] = &[1, 2, 3];
    if a.len() == 0 {
        return 0;
    }
    return unsafe { *a.as_ptr().add(i % a.len() + 1) };
}

fn integer_overflow(x: i32) -> i32 {
    if x <= i32::MAX - 100 {
        let add_num = |mut x: i32, z: i64| x += z as i32;
        add_num(x, 1);
        // overflow
        add_num(x, 101);
    }
    x
}

fn panic() {
    if kani::any() {
        panic!();
    }
}

#[kani::proof]
fn oob_safe_array_access_harness() {
    oob_safe_array_access(kani::any());
}

#[kani::proof]
fn oob_unsafe_array_access_harness() {
    oob_unsafe_array_access(kani::any());
}

#[kani::proof]
fn integer_overflow_harness() {
    integer_overflow(kani::any());
}

#[kani::proof]
fn panic_harness() {
    panic();
}
