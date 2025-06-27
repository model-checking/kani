// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! To run this test, do
//! kani main.rs -- lib.c

// kani-flags: -Z c-ffi --c-lib tests/kani/ForeignItems/lib.c

#[repr(C)]
pub struct Foo {
    i: u32,
    c: u8,
}

#[repr(C)]
pub struct Foo2 {
    i: u32,
    c: u8,
    i2: u32,
}

// https://doc.rust-lang.org/reference/items/external-blocks.html
// https://doc.rust-lang.org/nomicon/ffi.html
extern "C" {
    static mut S: u32;

    fn update_static();
    fn takes_int(i: u32) -> u32;
    fn takes_ptr(p: &u32) -> u32;
    fn takes_ptr_option(p: Option<&u32>) -> u32;
    fn mutates_ptr(p: &mut u32);
    #[link_name = "name_in_c"]
    fn name_in_rust(i: u32) -> u32;
    fn takes_struct(f: Foo) -> u32;
    fn takes_struct_ptr(f: &Foo) -> u32;
    fn takes_struct2(f: Foo2) -> u32;
    fn takes_struct_ptr2(f: &Foo2) -> u32;
}

#[kani::proof]
fn main() {
    unsafe {
        assert!(S == 12);
        update_static();
        assert!(S == 13);

        assert!(takes_int(1) == 3);
        assert!(takes_ptr(&5) == 7);

        let mut p = 17;
        mutates_ptr(&mut p);
        assert!(p == 16);

        assert!(name_in_rust(2) == 4);

        let f = Foo { i: 12, c: 7 };
        assert!(takes_struct_ptr(&f) == 19);
        assert!(takes_struct(f) == 19);

        let f2 = Foo2 { i: 12, c: 7, i2: 8 };
        // f2.i + f2.c
        assert!(takes_struct_ptr2(&f2) == 19);
        // f2.i + f2.i2
        assert!(takes_struct2(f2) == 20);
    }
}
