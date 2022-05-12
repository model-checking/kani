// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test relation comparison for vtable fat pointers fail due to unstable behavior.
use std::rc::Rc;

trait Dummy {}
impl Dummy for u8 {}

struct TestData {
    #[allow(dead_code)]
    array: Rc<[u8; 10]>,
    smaller_ptr: *const dyn Dummy,
    bigger_ptr: *const dyn Dummy,
}

fn setup() -> TestData {
    let array = Rc::new([0u8; 10]);
    TestData { array: array.clone(), smaller_ptr: &array[0], bigger_ptr: &array[5] }
}

#[kani::proof]
fn check_lt() {
    let data = setup();
    assert!(data.smaller_ptr < data.bigger_ptr);
}

#[kani::proof]
fn check_le() {
    let data = setup();
    assert!(data.smaller_ptr <= data.bigger_ptr);
}

#[kani::proof]
fn check_gt() {
    let data = setup();
    assert!(data.bigger_ptr > data.smaller_ptr);
}

#[kani::proof]
fn check_ge() {
    let data = setup();
    assert!(data.bigger_ptr >= data.smaller_ptr);
}

#[kani::proof]
fn check_ne() {
    let data = setup();
    assert!(data.bigger_ptr != data.smaller_ptr);
}

#[kani::proof]
fn check_eq() {
    let data = setup();
    assert!(!(data.bigger_ptr == data.smaller_ptr));
}
