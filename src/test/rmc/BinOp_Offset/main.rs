// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
pub fn test_offset_array() {
    let s = ['a', 'b', 'c'];
    let ptr = s.as_ptr();

    unsafe {
        assert!(*ptr.offset(0) as char == 'a');
        assert!(*ptr.offset(1) as char == 'b');
        assert!(*ptr.offset(2) as char == 'c');
    }
}

pub fn test_offset_str() {
    let s: &str = "123";
    let ptr: *const u8 = s.as_ptr();

    unsafe {
        assert!(*ptr.offset(0) == b'1');
        assert!(*ptr.offset(1) == b'2');
        assert!(*ptr.offset(2) == b'3');
    }
}

pub fn main() {
    test_offset_array();
    test_offset_str()
}
