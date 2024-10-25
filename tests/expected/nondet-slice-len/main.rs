// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that non-det slices created using `kani::any_slice_of_array`
// assume any length up the specified maximum

#[kani::proof]
fn check_possible_slice_lengths() {
    let arr: [i32; 4] = kani::any();
    let s = kani::any_slice_of_array(&arr);
    kani::cover!(s.len() == 0);
    kani::cover!(s.len() == 1);
    kani::cover!(s.len() == 2);
    kani::cover!(s.len() == 3);
    kani::cover!(s.len() == 4);
}
