// kani-check-fail
// compile-flags: --edition 2018
#![allow(unused)]
fn main() {
enum Void {}

let res: Result<u32, Void> = Ok(0);

// Err doesn't exist anymore, so Ok is actually irrefutable.
let Ok(num) = res;
}