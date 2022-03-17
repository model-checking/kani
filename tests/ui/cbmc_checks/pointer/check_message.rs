// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check we don't print temporary variables as part of CBMC messages.

fn not_zero(p1: *const i32) {
    assert!(unsafe { *p1 != 0 });
}

#[kani::proof]
#[kani::proof]
fn main() {
    let mut ptr = 10 as *const i32;
    if kani::any() {
        let var1 = 0;
        ptr = &var1 as *const i32;
    }
    not_zero(ptr);
}

