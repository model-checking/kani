// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    assert!(x_plus_two_1(0) == 2);
    assert!(x_plus_two_2(1) == 3);
    assert!(x_plus_two_1(x_plus_two_2(0) + 1) == 5);
}
fn x_plus_two_1(x: u32) -> u32 {
    x + 2
}
fn x_plus_two_2(x: u32) -> u32 {
    let y = x + 1;
    {
        let z = 1;
        y + z
    }
}
