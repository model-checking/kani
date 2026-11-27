// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks

//! Casting a pointer to an array to a pointer to its element type, as `core`'s sorting code does
//! with its scratch buffer, c.f. <https://github.com/model-checking/kani/issues/5233>. Casts that
//! change what the memory may hold are still reported.

use std::mem::MaybeUninit;
use std::num::{NonZeroU8, NonZeroU16};
use std::ptr;

const FIVE: NonZeroU8 = NonZeroU8::new(5).unwrap();
const SEVEN: NonZeroU8 = NonZeroU8::new(7).unwrap();
const NINE: NonZeroU8 = NonZeroU8::new(9).unwrap();

#[kani::proof]
fn array_elem_cast_nonzero() {
    let mut storage = MaybeUninit::<[NonZeroU8; 4]>::uninit();
    let element = storage.as_mut_ptr() as *mut NonZeroU8;
    unsafe { element.write(NonZeroU8::MIN) };
    kani::cover!(true);
}

#[kani::proof]
fn array_elem_write_then_read() {
    let mut arr = [NonZeroU8::MIN; 4];
    let arr_ptr: *mut [NonZeroU8; 4] = &raw mut arr;
    let element = arr_ptr as *mut NonZeroU8;
    unsafe { element.add(2).write(SEVEN) };
    let read = arr;
    assert_eq!(read[2].get(), 7);
    kani::cover!(true);
}

#[kani::proof]
fn nested_array_elem_cast() {
    let mut grid = [[NonZeroU8::MIN; 2]; 2];
    let grid_ptr: *mut [[NonZeroU8; 2]; 2] = &raw mut grid;
    let element = grid_ptr as *mut NonZeroU8;
    unsafe { element.add(3).write(NINE) };
    let row = grid_ptr as *mut [NonZeroU8; 2];
    unsafe { row.write([FIVE; 2]) };
    let read = grid;
    assert_eq!(read[0][1].get(), 5);
    assert_eq!(read[1][1].get(), 9);
    kani::cover!(true);
}

/// Bytes written through the element pointer are still checked when the array is read.
#[kani::proof]
fn ub_invalid_byte_through_elem_ptr() {
    let mut arr = [NonZeroU8::MIN; 4];
    let arr_ptr: *mut [NonZeroU8; 4] = &raw mut arr;
    let element = arr_ptr as *mut NonZeroU8;
    unsafe { ptr::write_bytes(element.add(2), 0, 1) };
    let read = arr;
    std::hint::black_box(read);
}

/// A `NonZeroU8` written over a `bool` would be invalid.
#[kani::proof]
fn bool_array_to_nonzero() {
    let mut arr = [false; 2];
    let arr_ptr: *mut [bool; 2] = &raw mut arr;
    let element = arr_ptr as *mut NonZeroU8;
    std::hint::black_box(element);
}

/// A `NonZeroU16` may hold a zero byte, which isn't a valid `NonZeroU8`.
#[kani::proof]
fn nonzero_array_to_wider_nonzero() {
    let mut arr = [NonZeroU8::MIN; 4];
    let arr_ptr: *mut [NonZeroU8; 4] = &raw mut arr;
    let element = arr_ptr as *mut NonZeroU16;
    std::hint::black_box(element);
}
