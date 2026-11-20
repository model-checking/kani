// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check pointers to tuples whose last element is a slice. Kani only generated slice fat pointers
//! for slices, `str` and structs, so these crashed the compiler:
//! <https://github.com/model-checking/kani/issues/5026>.
#![feature(ptr_metadata)]

#[kani::proof]
fn check_cast_to_unsized_tuple() {
    let arr: [u32; 4] = [1, 2, 3, 4];
    let s: *const [u32] = std::ptr::slice_from_raw_parts(arr.as_ptr(), 3);
    let p = s as *const (u32, [u32]);
    let r = unsafe { &*p };
    assert_eq!(std::mem::size_of_val(r), 16);
    assert_eq!(r.0, 1);
    assert_eq!(r.1, [2, 3, 4]);
}

#[kani::proof]
fn check_from_raw_parts_unsized_tuple() {
    let arr: [u32; 4] = [1, 2, 3, 4];
    let p: *const (u32, [u32]) = std::ptr::from_raw_parts(arr.as_ptr(), 3);
    let r = unsafe { &*p };
    assert_eq!(r.0, 1);
    assert_eq!(r.1.len(), 3);
    assert_eq!(r.1[2], 4);
}

/// The slice tail starts at offset 3 while the tuple is 2-aligned, so this also relies on the
/// struct layout no longer padding after the tail (issue 5027).
#[kani::proof]
fn check_unsized_tuple_with_odd_tail_offset() {
    let buf: [u16; 4] = [0x0201, 0x0403, 0x0605, 0x0807];
    let s: *const [u8] = std::ptr::slice_from_raw_parts(buf.as_ptr() as *const u8, 5);
    let p = s as *const (u16, u8, [u8]);
    let r = unsafe { &*p };
    assert_eq!(std::mem::size_of_val(r), 8);
    assert_eq!(std::mem::align_of_val(r), 2);
    assert_eq!(r.0, 0x0201);
    assert_eq!(r.1, 3);
    assert_eq!(r.2.len(), 5);
    assert_eq!(r.2[4], 8);
}
