// compile-flags: --edition 2021
#![allow(unused)]
// A stack-allocated array
fn main() {
let array: [i32; 3] = [1, 2, 3];

// A heap-allocated array, coerced to a slice
let boxed_array: Box<[i32]> = Box::new([1, 2, 3]);
}