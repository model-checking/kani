// compile-flags: --edition 2021
#![allow(unused)]
fn bar(_: &i8) { }

fn main() {
    bar(&mut 42);
}