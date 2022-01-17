// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-verify-fail

#![feature(core_intrinsics)]
use std::intrinsics::{ctlz, cttz, cttz_nonzero};

fn main() {
    let v8 = 0b0011_1000_u8;
    let v16 = 0b0011_1000_0000_0000_u16;
    let v32 = 0b0011_1000_0000_0000_0000_0000_0000_0000_u32;

    let nttz8 = cttz(v8);
    let nttz16 = cttz(v16);
    let nttz32 = unsafe { cttz_nonzero(v32) };
    let num_leading = ctlz(v8);
    let num_trailing_nz = unsafe { cttz_nonzero(v8) };

    // fail because of https://github.com/model-checking/rmc/issues/26
    assert!(nttz8 == 3);
    assert!(nttz16 == 11);
    assert!(nttz32 == 27);
    assert!(num_trailing_nz == 3);
    assert!(num_leading == 2);
}
