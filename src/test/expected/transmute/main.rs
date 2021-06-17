// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let bitpattern = unsafe {
        std::mem::transmute::<f32, u32>(1.0)
    };
    assert!(bitpattern == 0x3F800000);

    let f = unsafe {
        let i :u32 = 0x3F800000;
        std::mem::transmute::<u32, f32>(i)
    };
    assert!(f == 1.0);
}
