// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    let [x, y @ .., z] = [1, 2, 3, 4];
    assert!(x == 1);
    assert!(y[0] == 2);
    assert!(y[1] == 3);
    assert!(z == 4);
}
