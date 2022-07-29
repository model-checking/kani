// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
#[track_caller]
fn f() {
    println!("{}", std::panic::Location::caller());
}
}