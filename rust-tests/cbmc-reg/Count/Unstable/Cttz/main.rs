// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]
use std::intrinsics::{cttz, cttz_nonzero};

fn main() {
    let uv8 = 0b0011_1000_u8;
    let uv16 = uv8 as u16;
    let uv32 = uv8 as u32;
    let uv64 = uv8 as u64;
    let iv8 = i8::MIN;
    let iv16 = i16::MIN;
    let iv32 = i32::MIN;
    let iv64 = i64::MIN;

    assert!(cttz(uv8) == 3);
    assert!(cttz(uv16) == 3);
    assert!(cttz(uv32) == 3);
    assert!(cttz(uv64) == 3);
    assert!(cttz(iv8) == 7);
    assert!(cttz(iv16) == 15);
    assert!(cttz(iv32) == 31);
    assert!(cttz(iv64) == 63);

    unsafe {
        assert!(cttz(uv8) == cttz_nonzero(uv8));
        assert!(cttz(uv16) == cttz_nonzero(uv16));
        assert!(cttz(uv32) == cttz_nonzero(uv32));
        assert!(cttz(uv64) == cttz_nonzero(uv64));
    }
}
