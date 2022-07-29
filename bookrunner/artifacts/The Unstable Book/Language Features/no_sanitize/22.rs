// compile-flags: --edition 2015
#![allow(unused)]
#![feature(no_sanitize)]

fn main() {
#[no_sanitize(address)]
fn foo() {
  // ...
}
}