// kani-check-fail
// compile-flags: --edition 2015
#![allow(unused)]
#![feature(rustc_attrs)]

fn main() {
#[rustc_layout(abi, size)]
pub enum X {
    Y(u8, u8, u8),
    Z(isize),
}
}