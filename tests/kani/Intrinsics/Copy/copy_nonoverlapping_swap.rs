// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test the swap example from
// https://doc.redox-os.org/std/std/intrinsics/fn.copy_nonoverlapping.html
use std::mem;
use std::ptr;

#[kani::proof]
fn test_swap() {
    let mut x = 12;
    let mut y = 13;
    swap(&mut x, &mut y);
    assert!(x == 13);
    assert!(y == 12);
}

fn swap<T>(x: &mut T, y: &mut T) {
    unsafe {
        // Give ourselves some scratch space to work with
        let mut t: T = mem::uninitialized();

        // Perform the swap, `&mut` pointers never alias
        ptr::copy_nonoverlapping(x, &mut t, 1);
        ptr::copy_nonoverlapping(y, x, 1);
        ptr::copy_nonoverlapping(&t, y, 1);

        // y and t now point to the same thing, but we need to completely forget `tmp`
        // because it's no longer relevant.
        mem::forget(t);
    }
}
