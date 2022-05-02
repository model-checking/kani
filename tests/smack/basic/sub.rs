// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @expect error
// kani-verify-fail

pub fn main() {
    let a = 2;
    let b = 3;
    assert!(b - a != 1);
}
