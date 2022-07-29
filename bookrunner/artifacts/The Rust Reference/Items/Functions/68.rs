// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
fn first((value, _): (i32, i32)) -> i32 { value }
}