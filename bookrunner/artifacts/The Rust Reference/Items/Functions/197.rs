// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
extern fn new_i32() -> i32 { 0 }
let fptr: extern fn() -> i32 = new_i32;
}