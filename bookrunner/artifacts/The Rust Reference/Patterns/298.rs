// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
struct Person {
   name: String,
   age: u8,
}
let person = Person{ name: String::from("John"), age: 23 };
// `name` is moved from person and `age` referenced
let Person { name, ref age } = person;
}