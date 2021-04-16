// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let s: &str = "123";
    let ptr: *const u8 = s.as_ptr();

    unsafe {
        assert!(*ptr.offset(1) == b'2');
        assert!(*ptr.offset(2) == b'3');
    }
}
