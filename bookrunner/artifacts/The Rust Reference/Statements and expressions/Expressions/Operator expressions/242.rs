// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
let a = 1;
let b = 1;
a == b;
// is equivalent to
::std::cmp::PartialEq::eq(&a, &b);
}