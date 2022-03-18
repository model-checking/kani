// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::ptr::write;

#[kani::proof]
fn main() {
    let mut var = 1;
    unsafe {
        write(&mut var, 10);
    }
    assert_eq!(var, 10);
}
