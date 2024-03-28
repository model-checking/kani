// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
use std::any::Any;
type T<'a> = &'a (dyn Any + Send);
}