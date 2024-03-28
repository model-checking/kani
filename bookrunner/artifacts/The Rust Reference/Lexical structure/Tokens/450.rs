// kani-check-fail
// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
let example = ("dog", "cat", "horse");
let dog = example.0;
let cat = example.1;
// The following examples are invalid.
let cat = example.01;  // ERROR no field named `01`
let horse = example.0b10;  // ERROR no field named `0b10`
}