// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
pub fn main() {
    let x: u32 = rmc::nondet();
    if x < u32::MAX >> 1 {
        let y = x * 2;

        assert!(y % 2 == 0);
        assert!(y % 2 != 3);
    }
}
