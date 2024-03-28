// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
use std::fmt::Display;
fn foo(x: &u32) -> &dyn Display {
    x
}
}