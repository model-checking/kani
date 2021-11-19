// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let x: u32 = unsafe { rmc::nondet() };
    if x < u32::MAX >> 1 {
        let y = x * 2;

        assert!(y % 2 == 0);
        assert!(y % 2 != 3);
    }
}
