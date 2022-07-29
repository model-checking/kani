// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
let x: &Option<i32> = &Some(3);
if let Some(y) = x {
    // y was converted to `ref y` and its type is &i32
}
}