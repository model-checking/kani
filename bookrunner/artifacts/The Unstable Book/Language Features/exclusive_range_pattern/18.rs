// compile-flags: --edition 2015
#![allow(unused)]
#![feature(exclusive_range_pattern)]
fn main() {
let x = 5;
    match x {
        0..10 => println!("single digit"),
        10 => println!("ten isn't part of the above range"),
        _ => println!("nor is everything else.")
    }
}