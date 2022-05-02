// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that Kani reports out-of-bound accesses on a non-det slice
// created using kani::slice::any_slice

#[kani::proof]
fn check_out_of_bounds() {
    let mut bytes = kani::slice::any_slice::<u8, 5>();
    let val = unsafe { *bytes.get_slice().as_ptr().add(4) };
    kani::assume(val != 0);
    assert_ne!(val, 0);
}
