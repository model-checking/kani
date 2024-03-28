// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
#[repr(C)]
struct ThreeInts {
    first: i16,
    second: i8,
    third: i32
}
}