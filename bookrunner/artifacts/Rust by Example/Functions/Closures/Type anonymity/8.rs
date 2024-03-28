// compile-flags: --edition 2015
#![allow(unused)]
// `F` must be generic.
fn main() {
fn apply<F>(f: F) where
    F: FnOnce() {
    f();
}
}