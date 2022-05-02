// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that lexicographic comparison is handled correctly

#[kani::proof]
fn check_lexicographic_cmp() {
    assert!([1, 2, 3] < [1, 3, 4]);
    assert!("World" >= "Hello");
}
