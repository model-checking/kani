// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test is to check if the description for undefined functions has been updated to "Function with missing definition is unreachable"

// TODO: Missing functions produce non-informative property descriptions
// https://github.com/model-checking/kani/issues/1271
#[kani::proof]
fn main() {
    let x = String::from("foo");
    let y = x.clone();
    assert_eq!("foo", y);
}
