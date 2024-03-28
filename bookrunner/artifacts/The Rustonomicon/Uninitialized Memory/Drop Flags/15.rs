// compile-flags: --edition 2018
#![allow(unused)]
fn main() {
let mut x = Box::new(0); // let makes a fresh variable, so never need to drop
let y = &mut x;
*y = Box::new(1); // Deref assumes the referent is initialized, so always drops
}