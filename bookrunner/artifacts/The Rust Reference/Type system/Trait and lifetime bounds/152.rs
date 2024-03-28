// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
fn call_on_ref_zero<F>(f: F) where F: for<'a> Fn(&'a i32) {
    let zero = 0;
    f(&zero);
}
}