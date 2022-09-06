// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// This test checks the result of using Iterator::flat_map. We had some projection
// issues with this in the past.
// This currently fails due to missing core::str::count::char_count_general_case
// std function:
// https://github.com/model-checking/kani/issues/1213

#[kani::proof]
#[kani::unwind(4)]
fn check_flat_map_len() {
    let hello = ["Hi", "!"];
    let length = hello.iter().flat_map(|s| s.chars()).count();
    assert_eq!(length, 3);
}
