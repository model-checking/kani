// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[repr(packed)]
struct Packed {
    _padding: u8,
    unaligned: u32,
}

#[kani::proof]
fn main() {
    let packed: Packed = unsafe { std::mem::zeroed() };
    unsafe {
        let x = std::mem::transmute::<u32, u32>(packed.unaligned);
        assert_eq!(x, 0);
    }
}
