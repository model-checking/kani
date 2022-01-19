// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let arr = [1, 2, 3];
    // s is a slice (&[i32])
    let [s @ ..] = &arr[..];
    assert!(s[0] == 1);
    assert!(s[1] == 2);
}
