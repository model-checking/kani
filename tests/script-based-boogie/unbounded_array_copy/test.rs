// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A test that demonstrates unbounded verification of array-based programs.
//! The test uses `any_array` which creates arrays with non-deterministic
//! content and length.
//! The `src` array is copied into the `dst` array up to the minimum length of
//! the two arrays.

#[kani::proof]
fn copy() {
    let src = kani::array::any_array::<i32>();
    let mut dst = kani::array::any_array::<i32>();
    let src_len: usize = src.len();
    let dst_len: usize = dst.len();

    // copy as many elements as possible of `src` to `dst`
    let mut i: usize = 0;
    // Loop invariant: forall j: usize :: j < i => dst[j] == src[j])
    while i < src_len && i < dst_len {
        dst[i] = src[i];
        i = i + 1;
    }

    // check that the data was copied
    i = 0;
    while i < src_len && i < dst_len {
        kani::assert(dst[i] == src[i], "element doesn't have the correct value");
        i = i + 1;
    }
}
