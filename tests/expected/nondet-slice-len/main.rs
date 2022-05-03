// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that non-det slices created using kani::slice::any_slice can
// assume any length up the specified maximum

#[kani::proof]
fn check_possible_slice_lengths() {
    let s = kani::slice::any_slice::<i32, 4>();
    assert!(s.len() != 0);
    assert!(s.len() != 1);
    assert!(s.len() != 2);
    assert!(s.len() != 3);
    assert!(s.len() != 4);
}
