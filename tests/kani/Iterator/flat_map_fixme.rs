// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// This test checks the result of using Iterator::flat_map. We had some projection
// issues with this in the past.
//
// kani-flags: --unwind 3

#[kani::proof]
#[kani::unwind(3)]
pub fn check_flat_map_char() {
    let hi = ["H", "i"];
    let hi_flat = hi.iter().flat_map(|s| s.chars());
    assert_eq!(hi_flat.len(), 2);
}
