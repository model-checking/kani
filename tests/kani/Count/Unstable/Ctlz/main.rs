// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]
use std::intrinsics::{ctlz, ctlz_nonzero};

fn main() {
    let uv8 = 0b0011_1000_u8;
    let uv16 = uv8 as u16;
    let uv32 = uv8 as u32;
    let uv64 = uv8 as u64;
    let iv8 = i8::MIN;
    let iv16 = i16::MIN;
    let iv32 = i32::MIN;
    let iv64 = i64::MIN;

    assert!(ctlz(uv8) == 2);
    assert!(ctlz(uv16) == 10);
    assert!(ctlz(uv32) == 26);
    assert!(ctlz(uv64) == 58);
    assert!(ctlz(iv8) == 0);
    assert!(ctlz(iv16) == 0);
    assert!(ctlz(iv32) == 0);
    assert!(ctlz(iv64) == 0);

    unsafe {
        assert!(ctlz(uv8) == ctlz_nonzero(uv8));
        assert!(ctlz(uv16) == ctlz_nonzero(uv16));
        assert!(ctlz(uv32) == ctlz_nonzero(uv32));
        assert!(ctlz(uv64) == ctlz_nonzero(uv64));
    }
}
