// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
struct Car;
struct Computer;
struct Person {
    name: String,
    car: Option<Car>,
    computer: Option<Computer>,
    age: u8,
}
let person = Person {
    name: String::from("John"),
    car: Some(Car),
    computer: None,
    age: 15,
};
if let
    Person {
        car: Some(_),
        age: person_age @ 13..=19,
        name: ref person_name,
        ..
    } = person
{
    println!("{} has a car and is {} years old.", person_name, person_age);
}
}