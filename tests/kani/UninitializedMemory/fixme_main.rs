// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-verify-fail

// This tests demonstrates that Kani is unable to detect reads from
// uninitialized memory, which is considered undefined behavior.
// More details in https://github.com/model-checking/kani/issues/920

#[kani::proof]
fn main() {
    let mut v: Vec<u8> = Vec::with_capacity(8);
    unsafe {
        v.set_len(3);
    }
    let _b = v[0]; //< reading uninitialized memory
}
