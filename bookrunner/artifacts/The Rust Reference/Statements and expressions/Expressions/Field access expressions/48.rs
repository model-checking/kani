// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
struct A { f1: String, f2: String, f3: String }
let mut x: A;
x = A {
    f1: "f1".to_string(),
    f2: "f2".to_string(),
    f3: "f3".to_string()
};
let a: &mut String = &mut x.f1; // x.f1 borrowed mutably
let b: &String = &x.f2;         // x.f2 borrowed immutably
let c: &String = &x.f2;         // Can borrow again
let d: String = x.f3;           // Move out of x.f3
}