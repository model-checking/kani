// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test uses `kani::slice::any_slice_of_array` with `i32`

// kani-flags: --default-unwind 6

#[kani::proof]
fn check_any_slice_i32() {
    let a: [i32; 5] = kani::any();
    let s = kani::slice::any_slice_of_array(&a);
    s.iter().for_each(|x| kani::assume(*x < 10 && *x > -20));
    let sum = s.iter().fold(0, |acc, x| acc + x);
    assert!(sum <= 45); // 9 * 5
    assert!(sum >= -95); // 19 * 5
}
