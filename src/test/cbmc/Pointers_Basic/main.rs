// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let x = 3;
    let y = &x;
    let mut z = *y;

    assert!(z == 3);
}
