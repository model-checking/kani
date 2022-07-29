// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
#[track_caller]
fn f() {
    println!("{}", std::panic::Location::caller());
}
#[track_caller]
fn g() {
    println!("{}", std::panic::Location::caller());
    f();
}

fn calls_g() {
    g(); // <-- g() prints this location twice, once itself and once from f()
}
}