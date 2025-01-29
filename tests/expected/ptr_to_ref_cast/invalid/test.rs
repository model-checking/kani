// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test case checks the usage of slices of slices (&[&[T]]).
use std::mem::size_of_val;

/// Structure with a raw string (i.e.: [char]).
struct MyStr {
    header: u16,
    data: str,
}

impl MyStr {
    /// This creates a MyStr from a byte slice.
    fn new(original: &mut String) -> &mut Self {
        let buf = original.get_mut(..).unwrap();
        assert!(size_of_val(buf) > 2, "This requires at least 2 bytes");
        let unsized_len = buf.len() - 2;
        let ptr = std::ptr::slice_from_raw_parts_mut(buf.as_mut_ptr(), unsized_len);
        unsafe { &mut *(ptr as *mut Self) }
    }
}

#[kani::proof]
fn sanity_check_my_str() {
    let mut buf = String::from("123456");
    let my_str = MyStr::new(&mut buf);
    my_str.header = 0;

    assert_eq!(size_of_val(my_str), 6);
    assert_eq!(my_str.data.len(), 4);
    assert_eq!(my_str.data.chars().nth(0), Some('3'));
    assert_eq!(my_str.data.chars().nth(3), Some('6'));
}

#[kani::proof]
fn check_slice_my_str() {
    let mut buf_0 = String::from("000");
    let mut buf_1 = String::from("001");
    let my_slice = &[MyStr::new(&mut buf_0), MyStr::new(&mut buf_1)];
    assert_eq!(my_slice.len(), 2);

    assert_eq!(my_slice[0].data.len(), 1);
    assert_eq!(my_slice[1].data.len(), 1);

    assert_eq!(my_slice[0].data.chars().nth(0), Some('0'));
    assert_eq!(my_slice[1].data.chars().nth(0), Some('1'));
}

#[kani::proof]
fn check_size_of_val() {
    let mut buf_0 = String::from("000");
    let mut buf_1 = String::from("001");
    let my_slice = &[MyStr::new(&mut buf_0), MyStr::new(&mut buf_1)];
    assert_eq!(size_of_val(my_slice), 32); // Slice of 2 fat pointers.
    assert_eq!(size_of_val(my_slice[0]), 4); // Size of a fat pointer.
    assert_eq!(size_of_val(&my_slice[0].data), 1); // Size of str.
}
