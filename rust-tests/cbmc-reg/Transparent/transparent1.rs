// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn main() {
    let mut x: u32 = 4;
    let pointer0: std::ptr::NonNull<u32> = std::ptr::NonNull::new(&mut x).unwrap();
    let y = unsafe { *pointer0.as_ptr() };
    assert!(y == 4);
}
