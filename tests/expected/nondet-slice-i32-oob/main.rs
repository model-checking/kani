// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that Kani reports out-of-bound accesses on a non-det slice
// created using kani::slice::any_slice

#[kani::proof]
fn check_out_of_bounds() {
    let bytes = kani::slice::any_slice::<i32, 8>();
    let val = unsafe { *bytes.get_slice().as_ptr().offset(1) };
    assert_eq!(val - val, 0);
}
