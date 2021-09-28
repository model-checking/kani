// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(asm)]

pub fn main() {
    unsafe {
        asm!("nop");
    }
}
