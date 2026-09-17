// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that Kani can codegen and verify functions that take an unsized argument
//! *by value* via the `unsized_fn_params` feature (e.g. `fn f(self: [T])` and
//! `fn f(self: str)`). Such arguments are passed as fat pointers, so both the
//! length/metadata (`self.len()`, `self.as_bytes()`) and the data (`self[i]`)
//! must be recovered correctly.

#![feature(unsized_fn_params)]

trait SumSlice {
    fn sum(self) -> u32;
}

impl SumSlice for [u32] {
    fn sum(self) -> u32 {
        // `len()` exercises the fat-pointer metadata; indexing exercises the data pointer.
        assert_eq!(self.len(), 3);
        self[0] + self[1] + self[2]
    }
}

trait FirstByte {
    fn first_byte(self) -> u8;
}

impl FirstByte for str {
    fn first_byte(self) -> u8 {
        self.as_bytes()[0]
    }
}

#[kani::proof]
fn check_unsized_slice_arg() {
    let boxed: Box<[u32]> = Box::new([1u32, 2, 3]);
    assert_eq!((*boxed).sum(), 6);
}

#[kani::proof]
fn check_unsized_str_arg() {
    let boxed: Box<str> = String::from("hi").into_boxed_str();
    assert_eq!((*boxed).first_byte(), b'h');
}
