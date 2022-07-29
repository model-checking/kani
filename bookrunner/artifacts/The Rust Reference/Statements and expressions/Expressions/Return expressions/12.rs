// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
fn max(a: i32, b: i32) -> i32 {
    if a > b {
        return a;
    }
    return b;
}
}