// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
let x = 6;
assert_eq!(-x, -6);
assert_eq!(!x, -7);
assert_eq!(true, !false);
}