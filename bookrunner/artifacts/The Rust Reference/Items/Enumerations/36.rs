// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
enum Animal {
    Dog,
    Cat,
}

let mut a: Animal = Animal::Dog;
a = Animal::Cat;
}