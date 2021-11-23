// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let mut z = 1;
    for i in 1..4 {
        z *= i;
    }
    assert!(z == 6);
}
