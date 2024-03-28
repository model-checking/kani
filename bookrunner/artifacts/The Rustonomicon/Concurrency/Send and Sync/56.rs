// compile-flags: --edition 2018
#![allow(unused)]
#![feature(negative_impls)]

// I have some magic semantics for some synchronization primitive!
fn main() {
struct SpecialThreadToken(u8);

impl !Send for SpecialThreadToken {}
impl !Sync for SpecialThreadToken {}
}