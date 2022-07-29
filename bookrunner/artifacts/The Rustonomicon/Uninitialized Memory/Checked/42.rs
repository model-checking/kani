// kani-check-fail
// compile-flags: --edition 2018
#![allow(unused)]
fn main() {
    let x: i32;
    if true {
        x = 1;
    }
    println!("{}", x);
}