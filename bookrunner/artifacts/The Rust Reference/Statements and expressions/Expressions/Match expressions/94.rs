// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
let maybe_digit = Some(0);
fn process_digit(i: i32) { }
fn process_other(i: i32) { }
let message = match maybe_digit {
    Some(x) if x < 10 => process_digit(x),
    Some(x) => process_other(x),
    None => panic!(),
};
}