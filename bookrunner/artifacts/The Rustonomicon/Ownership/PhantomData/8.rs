// kani-check-fail
// compile-flags: --edition 2018
#![allow(unused)]
fn main() {
struct Iter<'a, T: 'a> {
    ptr: *const T,
    end: *const T,
}
}