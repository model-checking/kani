// kani-check-fail
// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
fn temp() {}
trait Use { fn use_temp(&self) -> &Self { self } }
impl Use for () {}
// The temporary that stores the result of `temp()` only lives until the
// end of the let statement in these cases.

let x = Some(&temp());         // ERROR
let x = (&temp()).use_temp();  // ERROR
x;
}