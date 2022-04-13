// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks sub-array slice pattern, which is currently not supported:
// https://github.com/model-checking/kani/issues/707

#[kani::proof]
fn main() {
    let [x, y @ .., z] = [1, 2, 3, 4];
    assert_eq!(x, 1);
    assert_eq!(y, [2, 3]);
    assert_eq!(z, 4);
}
