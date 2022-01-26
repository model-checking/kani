// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]
use std::intrinsics::{rotate_left, rotate_right};

fn main() {
    let vl8 = 0b0011_1000_u8;
    let vl16 = 0b0011_1000_0000_0000_u16;
    let vl32 = 0b0011_1000_0000_0000_0000_0000_0000_0000_u32;
    let vl64 =
        0b0011_1000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_u64;

    let vr8 = 0b0000_1110_u8;
    let vr16 = 0b0000_0000_0000_1110_u16;
    let vr32 = 0b0000_0000_0000_0000_0000_0000_0000_1110_u32;
    let vr64 =
        0b0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_1110_u64;

    let rol8 = rotate_left(vl8, 3);
    let rol16 = rotate_left(vl16, 3);
    let rol32 = rotate_left(vl32, 3);
    let rol64 = rotate_left(vl64, 3);

    let ror8 = rotate_right(vr8, 3);
    let ror16 = rotate_right(vr16, 3);
    let ror32 = rotate_right(vr32, 3);
    let ror64 = rotate_right(vr64, 3);

    assert!(rol8 == 0b1100_0001_u8);
    assert!(rol16 == 0b1100_0000_0000_0001_u16);
    assert!(rol32 == 0b1100_0000_0000_0000_0000_0000_0000_0001_u32);
    assert!(rol64 == 0b1100_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0001_u64);

    assert!(ror8 == 0b1100_0001_u8);
    assert!(ror16 == 0b1100_0000_0000_0001_u16);
    assert!(ror32 == 0b1100_0000_0000_0000_0000_0000_0000_0001_u32);
    assert!(ror64 == 0b1100_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0001_u64);
}
