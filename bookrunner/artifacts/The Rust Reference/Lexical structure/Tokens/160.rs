// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
let a = "foobar";
let b = "foo\
         bar";

assert_eq!(a,b);
}