// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let mut a = 5;
    let mut c = &mut a;
    add_two(c);
    assert!(*c == 7 && a == 7);
    let mut b = 10;
    c = &mut b;
    assert!(*c == 10 && b == 10);
}

fn add_two(a: *mut u32) {
    unsafe {
        *a += 2;
    }
}
