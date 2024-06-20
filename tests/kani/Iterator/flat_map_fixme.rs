// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// This test checks the result of using Iterator::flat_map. We had some projection
// issues with this in the past.

#[kani::proof]
#[kani::unwind(3)]
pub fn check_flat_map_char() {
    let hi = ["H", "i"];
    let mut hi_flat = hi.iter().flat_map(|s| s.chars());
    assert_eq!(hi_flat.next(), Some('H'));
    assert_eq!(hi_flat.next(), Some('i'));
    assert_eq!(hi_flat.next(), None);
}
