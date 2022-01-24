// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let mut x = 1;
    add_two(&mut x);
    assert!(x == 3);
}

fn add_two(x: *mut u32) {
    unsafe {
        *x += 2;
    }
}
