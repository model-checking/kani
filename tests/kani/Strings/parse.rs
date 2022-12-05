// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test is to check how Kani handle some std functions, such as parse.
//! This test used to trigger a missing function before the MIR Linker.

#[kani::proof]
#[kani::unwind(3)]
pub fn main() {
    let strings = vec!["tofu", "93"];
    let numbers: Vec<_> = strings.into_iter().filter_map(|s| s.parse::<i32>().ok()).collect();
    assert_eq!(numbers.len(), 1);
    assert_eq!(numbers[0], 93);
}
