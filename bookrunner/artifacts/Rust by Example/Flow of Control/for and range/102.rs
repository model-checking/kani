// compile-flags: --edition 2015
// kani-flags: --enable-unstable --cbmc-args --unwind 7
#![allow(unused)]
fn main() {
    let mut names = vec!["Bob", "Frank", "Ferris"];

    for name in names.iter_mut() {
        *name = match name {
            &mut "Ferris" => "There is a rustacean among us!",
            _ => "Hello",
        }
    }

    println!("names: {:?}", names);
}