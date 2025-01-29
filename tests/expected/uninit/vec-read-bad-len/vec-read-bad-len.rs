// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

use std::ops::Index;

/// Checks that Kani catches an attempt to read uninitialized memory from a vector with bad length.
#[kani::proof]
fn check_vec_read_bad_len() {
    let mut v: Vec<u8> = Vec::with_capacity(10);
    unsafe {
        v.set_len(5); // even though length is now 5, vector is still uninitialized
    }
    let uninit = v.index(0); // ~ERROR: reading from unitialized memory is UB.
}
