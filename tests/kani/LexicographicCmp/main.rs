// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that lexicographic comparison is handled correctly

#[kani::proof]
fn main() {
    assert!([1, 2, 3] < [1, 3, 4]);
    assert!("World" >= "Hello");
}
