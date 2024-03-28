// compile-flags: --edition 2015
#![allow(unused)]
#![crate_type = "lib"]

fn main() {
use std::ptr;

pub fn foo() {
    // this function doesn't use the stack
}

pub fn bar() {
    let xs = [0u32; 2];

    // force LLVM to allocate `xs` on the stack
    unsafe { ptr::read_volatile(&xs.as_ptr()); }
}
}