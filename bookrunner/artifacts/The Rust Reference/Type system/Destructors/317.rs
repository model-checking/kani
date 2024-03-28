// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
fn temp() {}
trait Use { fn use_temp(&self) -> &Self { self } }
impl Use for () {}
// The temporary that stores the result of `temp()` lives in the same scope
// as x in these cases.
let x = &temp();
let x = &temp() as &dyn Send;
let x = (&*&temp(),);
let x = { [Some { 0: &temp(), }] };
let ref x = temp();
let ref x = *&temp();
x;
}