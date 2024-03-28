// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
struct Person {
   name: String,
   age: u8,
}
let value = Person { name: String::from("John"), age: 23 };
if let Person {name: ref person_name, age: 18..=150 } = value { }
}