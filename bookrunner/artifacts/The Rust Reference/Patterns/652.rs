// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
struct Struct {
   a: i32,
   b: char,
   c: bool,
}
let struct_value = Struct{a: 10, b: 'X', c: false};

let Struct{a: x, b: y, c: z} = struct_value;          // destructure all fields
}