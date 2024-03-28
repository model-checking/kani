// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
let x = &7;
assert_eq!(*x, 7);
let y = &mut 9;
*y = 11;
assert_eq!(*y, 11);
}