// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn main() {
    let x: u128 = u128::MAX;
    let x2: u128 = {
        // u128::MAX = 2^128 - 1;
        //           = (2^64 - 1) * (2^64 + 1)
        //           = u64::MAX + (u64::MAX + 2)
        let u64_max = u64::MAX;
        let u64_max_u128 = u64_max as u128;
        u64_max_u128 * (u64_max_u128 + 2)
    };
    assert!(x == x2);

    let y: i128 = i128::MAX;
    let y2: i128 = {
        // i128::MAX = 2^127 - 1
        //           = (2^63 * 2^63) + (2^63 * 2^63 - 1)
        let u64_2_63 = 2u64.pow(63);
        let i128_2_63 = u64_2_63 as i128;
        let i128_2_64 = i128_2_63 * i128_2_63;
        i128_2_64 + (i128_2_64 - 1)
    };
    assert!(y == y2);

    let z: i128 = i128::MIN;
    let z2: i128 = {
        // i128::MAX = -2^127
        //           = 0 - (2^63 * 2^63) - (2^63 * 2^63)
        let u64_2_63 = 2u64.pow(63);
        let i128_2_63 = u64_2_63 as i128;
        let i128_2_64 = i128_2_63 * i128_2_63;
        0i128 - i128_2_64 - i128_2_64
    };
    assert!(z == z2);
}
