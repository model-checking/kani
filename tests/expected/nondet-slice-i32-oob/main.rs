// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that Kani reports out-of-bound accesses on a non-det slice
// created using `kani::any_slice_of_array`

#[kani::proof]
fn check_out_of_bounds() {
    let arr: [i32; 8] = kani::any();
    let bytes = kani::any_slice_of_array(&arr);
    let val = unsafe { *bytes.as_ptr().offset(1) };
    assert_eq!(val - val, 0);
}
