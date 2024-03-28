// compile-flags: --edition 2015
#![allow(unused)]
fn main() {
fn next_birthday(current_age: Option<u8>) -> Option<String> {
	// If `current_age` is `None`, this returns `None`.
	// If `current_age` is `Some`, the inner `u8` gets assigned to `next_age`
    let next_age: u8 = current_age?;
    Some(format!("Next year I will be {}", next_age))
}
}