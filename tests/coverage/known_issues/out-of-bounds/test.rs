// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test should check that the return in `get` is `UNCOVERED`. However, the
//! coverage results currently report that the whole function is `COVERED`,
//! likely due to <https://github.com/model-checking/kani/issues/3441>

fn get(s: &[i16], index: usize) -> i16 {
    s[index]
}

#[kani::proof]
fn main() {
    get(&[7, -83, 19], 15);
}
