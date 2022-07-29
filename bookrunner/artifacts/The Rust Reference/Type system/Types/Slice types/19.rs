// compile-flags: --edition 2021
#![allow(unused)]
// A heap-allocated array, coerced to a slice
fn main() {
let boxed_array: Box<[i32]> = Box::new([1, 2, 3]);

// A (shared) slice into an array
let slice: &[i32] = &boxed_array[..];
}