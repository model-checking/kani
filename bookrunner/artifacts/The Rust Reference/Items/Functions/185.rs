// compile-flags: --edition 2021
#![allow(unused)]
// Declares a function with the "C" ABI
fn main() {
extern "C" fn new_i32() -> i32 { 0 }

// Declares a function with the "stdcall" ABI
#[cfg(target_arch = "x86_64")]
extern "stdcall" fn new_i32_stdcall() -> i32 { 0 }
}