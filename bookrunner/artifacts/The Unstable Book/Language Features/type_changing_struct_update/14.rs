// compile-flags: --edition 2015
#![allow(unused)]
#![allow(unused_variables, dead_code)]
#![feature(type_changing_struct_update)]

fn main () {
    struct Foo<T, U> {
        field1: T,
        field2: U,
    }

    let base: Foo<String, i32> = Foo {
        field1: String::from("hello"),
        field2: 1234,
    };
    let updated: Foo<f64, i32> = Foo {
        field1: 3.14,
        ..base
    };
}