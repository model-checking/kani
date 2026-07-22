// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test reference and pointer support in Strata backend

#[kani::proof]
fn test_reference() {
    let x: u32 = 42;
    let r: &u32 = &x;
    assert!(*r == 42);
}

#[kani::proof]
fn test_mutable_reference() {
    let mut x: u32 = 10;
    let r: &mut u32 = &mut x;
    *r = 20;
    assert!(x == 20);
}

#[kani::proof]
fn test_reference_to_struct() {
    struct Point { x: u32, y: u32 }

    let p = Point { x: 5, y: 10 };
    let r: &Point = &p;
    assert!(r.x == 5);
    assert!(r.y == 10);
}

#[kani::proof]
fn test_reference_parameter() {
    fn add_one(x: &mut u32) {
        *x = *x + 1;
    }

    let mut val: u32 = 5;
    add_one(&mut val);
    assert!(val == 6);
}

#[kani::proof]
fn test_reference_return() {
    fn get_ref(x: &u32) -> &u32 {
        x
    }

    let val: u32 = 100;
    let r = get_ref(&val);
    assert!(*r == 100);
}

#[kani::proof]
fn test_array_reference() {
    let arr: [u32; 3] = [1, 2, 3];
    let r: &[u32; 3] = &arr;
    assert!(r[0] == 1);
    assert!(r[1] == 2);
}

#[kani::proof]
fn test_multiple_references() {
    let x: u32 = 42;
    let r1: &u32 = &x;
    let r2: &u32 = &x;
    assert!(*r1 == *r2);
}
