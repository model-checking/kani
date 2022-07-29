// compile-flags: --edition 2018
#![allow(unused)]
fn main() {
unsafe fn kaboom(ptr: *const i32) -> i32 { *ptr }
}