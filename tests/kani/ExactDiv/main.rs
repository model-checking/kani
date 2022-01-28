// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]

fn main() {
    let a: u8 = 8;
    let b: u8 = 4;
    let i = unsafe { std::intrinsics::exact_div(a, b) };
    assert!(i == 2);
}
