// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    check_u32(4);
    check_u32(123);
    check_u32(119);
}
fn check_u32(x: u32) {
    if x % 2 == 0 {
        assert!(x < 119)
    } else if x % 3 == 0 {
        assert!(x > 119)
    } else {
        assert!(x == 119)
    }
}
