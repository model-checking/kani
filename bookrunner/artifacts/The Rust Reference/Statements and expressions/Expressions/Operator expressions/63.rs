// compile-flags: --edition 2021
#![allow(unused)]
// same meanings:
fn main() {
let a = &&  10;
let a = & & 10;

// same meanings:
let a = &&&&  mut 10;
let a = && && mut 10;
let a = & & & & mut 10;
}