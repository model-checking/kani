// compile-flags: --edition 2018
#![allow(unused)]
fn main() {
fn do_stuff<T: Clone>(value: &T) {
    let cloned = value.clone();
}
}