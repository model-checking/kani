// compile-flags: --edition 2021
#![allow(unused)]
// Examples of associated trait items with and without definitions.
fn main() {
trait Example {
    const CONST_NO_DEFAULT: i32;
    const CONST_WITH_DEFAULT: i32 = 99;
    type TypeNoDefault;
    fn method_without_default(&self);
    fn method_with_default(&self) {}
}
}