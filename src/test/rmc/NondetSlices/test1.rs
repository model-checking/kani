// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn check(slice: &[u8]) {
    let len = slice.len();
    assert!(len >= 0 && len <= 3);
    if len > 0 {
        let elem = slice[0];
        assert!(elem == 1 || elem == 2 || elem == 3);
    }
}

#[no_mangle]
fn main() {
    let arr = [1, 2, 3];
    let slice = rmc::slice::nondet_slice_of_array(&arr);
    check(slice);
}
