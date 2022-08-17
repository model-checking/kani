// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn init_int() {
    let a = [4u8; 6];
    let i: usize = kani::any();
    kani::assume(i < 6);
    assert_eq!(a[i], 4);
}

#[kani::proof]
fn init_option() {
    let a = [Some(4u8); 6];
    let i: usize = kani::any();
    kani::assume(i < 6);
    assert_eq!(a[i], Some(4));
}

#[kani::proof]
fn init_double_array() {
    let a = [Some(4u8); 6];
    let b = [a; 3];
    let i: usize = kani::any();
    kani::assume(i < 3);
    let j: usize = kani::any();
    kani::assume(j < 6);
    assert_eq!(b[i][j], Some(4));
}

#[repr(C)]
struct NonCopy {
    a: u8,
    b: u32,
    c: Option<u16>,
}

#[kani::proof]
fn init_array_of_non_copystruct() {
    let v = NonCopy { a: 1, b: 2, c: Some(3) };
    // If length is >1, Rust complains that the struct must be `Copy`.
    let a = [v; 1];
    let i: usize = kani::any();
    kani::assume(i < 1);
    assert_eq!(a[i].a, 1);
    assert_eq!(a[i].b, 2);
    assert_eq!(a[i].c, Some(3));
}

#[derive(Copy, Clone)]
struct Copyable {
    a: u8,
    b: u32,
    c: Option<u16>,
}

#[kani::proof]
fn init_array_of_struct() {
    let v = Copyable { a: 1, b: 2, c: Some(3) };
    let a = [v; 6];
    let i: usize = kani::any();
    kani::assume(i < 6);
    assert_eq!(a[i].a, 1);
    assert_eq!(a[i].b, 2);
    assert_eq!(a[i].c, Some(3));
}

#[repr(C)]
#[derive(Copy, Clone)]
struct ReprC {
    a: u8,
    b: u32,
    c: Option<u16>,
}

#[kani::proof]
fn init_array_of_repr_c_struct() {
    let v = ReprC { a: 1, b: 2, c: Some(3) };
    let a = [v; 6];
    let i: usize = kani::any();
    kani::assume(i < 6);
    assert_eq!(a[i].a, 1);
    assert_eq!(a[i].b, 2);
    assert_eq!(a[i].c, Some(3));
}

#[kani::proof]
fn mutate_array() {
    let mut a = [4u8; 6];
    a[2] = 1;
    assert_eq!(a[2], 1);
}
