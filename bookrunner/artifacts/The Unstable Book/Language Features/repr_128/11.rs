// compile-flags: --edition 2015
#![allow(unused)]
#![feature(repr128)]

fn main() {
#[repr(u128)]
enum Foo {
    Bar(u64),
}
}