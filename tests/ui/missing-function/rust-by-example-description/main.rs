// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --enable-unstable --cbmc-args --unwind 4 --object-bits 9
// This test is to check if the description for undefined functions has been updated to "Function with missing definition is unreachable"

// TODO: Missing functions produce non-informative property descriptions
// https://github.com/model-checking/kani/issues/1271
#![allow(unused)]
#[kani::proof]
pub fn main() {
    let strings = vec!["tofu", "93", "18"];
    let numbers: Vec<_> = strings.into_iter().filter_map(|s| s.parse::<i32>().ok()).collect();
    println!("Results: {:?}", numbers);
}
