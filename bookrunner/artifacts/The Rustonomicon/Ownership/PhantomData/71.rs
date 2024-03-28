// compile-flags: --edition 2018
#![allow(unused)]
fn main() {
use std::marker;

struct Vec<T> {
    data: *const T, // *const for variance!
    len: usize,
    cap: usize,
    _marker: marker::PhantomData<T>,
}
}