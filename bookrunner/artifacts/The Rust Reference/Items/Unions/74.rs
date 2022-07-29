// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
use std::mem::ManuallyDrop;
union MyUnion { f1: u32, f2: ManuallyDrop<String> }
let mut u = MyUnion { f1: 1 };

// These do not require `unsafe`.
u.f1 = 2;
u.f2 = ManuallyDrop::new(String::from("example"));
}