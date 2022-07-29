// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
extern "C" fn new_i32() -> i32 { 0 }
let fptr: extern "C" fn() -> i32 = new_i32;
}