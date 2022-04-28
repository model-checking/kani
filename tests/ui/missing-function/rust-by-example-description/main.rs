// compile-flags: --edition 2018
// kani-flags: --enable-unstable --cbmc-args --unwind 4 --object-bits 9

#![allow(unused)]
#[kani::proof]
pub fn main() {
    let strings = vec!["tofu", "93", "18"];
    let numbers: Vec<_> = strings
        .into_iter()
        .filter_map(|s| s.parse::<i32>().ok())
        .collect();
    println!("Results: {:?}", numbers);
}
