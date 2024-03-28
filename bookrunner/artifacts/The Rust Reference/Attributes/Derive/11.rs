// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
#[derive(PartialEq, Clone)]
struct Foo<T> {
    a: i32,
    b: T,
}
}