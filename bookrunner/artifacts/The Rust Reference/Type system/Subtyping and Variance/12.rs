// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
fn bar<'a>() {
    let s: &'static str = "hi";
    let t: &'a str = s;
}
}