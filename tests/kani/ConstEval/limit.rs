// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Ensure that constant propagation can deal with a large number.
// This test used to trigger https://github.com/rust-lang/rust/issues/103814

const LENGTH: usize = 131072;
const CONST: usize = {
    let data = [1; LENGTH];
    let mut idx = 0;
    while idx < data.len() {
        idx += data[idx];
    }
    idx
};

#[kani::proof]
fn check_eval() {
    assert_eq!(CONST, LENGTH);
}
