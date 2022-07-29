// compile-flags: --edition 2018
#![allow(unused)]
fn main() {
struct Foo<T, U> {
    count: u16,
    data1: T,
    data2: U,
}
}