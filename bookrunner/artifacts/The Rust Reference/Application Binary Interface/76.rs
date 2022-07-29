// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
#[no_mangle]
#[link_section = ".example_section"]
pub static VAR1: u32 = 1;
}