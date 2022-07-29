// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
let x = &mut 0;
// Usually a temporary would be dropped by now, but the temporary for `0` lives
// to the end of the block.
println!("{}", x);
}