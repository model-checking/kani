// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
union MyUnion { f1: u32, f2: f32 }

let u = MyUnion { f1: 1 };
let f = unsafe { u.f1 };
}