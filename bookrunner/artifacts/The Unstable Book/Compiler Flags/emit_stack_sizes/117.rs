// compile-flags: --edition 2015
#![allow(unused)]
// file: src/main.rs
use std::ptr;

#[inline(never)]
fn main() {
    let xs = [0u32; 2];

    // force LLVM to allocate `xs` on the stack
    unsafe { ptr::read_volatile(&xs.as_ptr()); }
}