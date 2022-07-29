// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
use std::io;
fn setup_the_thing() -> io::Result<i32> { Ok(1) }
fn do_the_thing(s: &i32) -> io::Result<()> { Ok(()) }
#[test]
fn test_the_thing() -> io::Result<()> {
    let state = setup_the_thing()?; // expected to succeed
    do_the_thing(&state)?;          // expected to succeed
    Ok(())
}
}