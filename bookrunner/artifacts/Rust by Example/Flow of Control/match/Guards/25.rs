// compile-flags: --edition 2015
#![allow(unused)]
fn main() {
    let number: u8 = 4;

    match number {
        i if i == 0 => println!("Zero"),
        i if i > 0 => println!("Greater than zero"),
        _ => println!("Fell through"), // This should not be possible to reach
    }
}