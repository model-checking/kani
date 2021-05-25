// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn main() {
    let address = 0x01234usize;
    let ptr = address as *mut i32;
    unsafe {
        // random address cannot be dereferenced from memory
        *ptr = 1;
    };
}
