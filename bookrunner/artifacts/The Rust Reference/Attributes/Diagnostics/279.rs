// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
#[must_use]
fn five() -> i32 { 5i32 }

// None of these violate the unused_must_use lint.
(five(),);
Some(five());
{ five() };
if true { five() } else { 0i32 };
match true {
    _ => five()
};
}