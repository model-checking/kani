// kani-check-fail
// compile-flags: --edition 2021
#![allow(unused)]
// Self: Sized traits are not object-safe.
fn main() {
trait TraitWithSize where Self: Sized {}

struct S;
impl TraitWithSize for S {}
let obj: Box<dyn TraitWithSize> = Box::new(S); // ERROR
}