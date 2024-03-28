// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
struct Struct {
   a: i32,
   b: char,
   c: bool,
}
let mut struct_value = Struct{a: 10, b: 'X', c: false};

match struct_value {
    Struct{a: 10, b: 'X', c: false} => (),
    Struct{a: 10, b: 'X', ref c} => (),
    Struct{a: 10, b: 'X', ref mut c} => (),
    Struct{a: 10, b: 'X', c: _} => (),
    Struct{a: _, b: _, c: _} => (),
}
}