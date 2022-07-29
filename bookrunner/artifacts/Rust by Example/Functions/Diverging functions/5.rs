// compile-flags: --edition 2015
#![allow(unused)]
fn main() {
fn foo() -> ! {
    panic!("This call never returns.");
}
}