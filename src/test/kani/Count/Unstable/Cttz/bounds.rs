// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-verify-fail
// cbmc-flags: --bounds-check

#![feature(core_intrinsics)]
use std::intrinsics::cttz_nonzero;

fn main() {
    let uv8: u8 = 0;
    let uv16: u16 = 0;
    let uv32: u32 = 0;
    let uv64: u64 = 0;
    let iv8: i8 = 0;
    let iv16: i16 = 0;
    let iv32: i32 = 0;
    let iv64: i64 = 0;

    // Each one of the following statements is expected to fail
    // because `cttz_nonzero` shows UB when the argument is 0
    unsafe {
        let _ = cttz_nonzero(uv8);
        let _ = cttz_nonzero(uv16);
        let _ = cttz_nonzero(uv32);
        let _ = cttz_nonzero(uv64);
        let _ = cttz_nonzero(iv8);
        let _ = cttz_nonzero(iv16);
        let _ = cttz_nonzero(iv32);
        let _ = cttz_nonzero(iv64);
    }
}
