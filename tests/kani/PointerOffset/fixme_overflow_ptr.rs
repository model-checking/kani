// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn check_thin_ptr() {
    let array = [0, 1, 2, 3, 4, 5, 6];
    let second_ptr: *const i32 = &array[3];
    unsafe {
        let before = second_ptr.sub(1);
        assert_eq!(*before, 2);
    }
}
