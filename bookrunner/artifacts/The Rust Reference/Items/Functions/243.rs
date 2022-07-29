// compile-flags: --edition 2021
#![allow(unused)]
// Source
fn main() {
async fn example(x: &str) -> usize {
    x.len()
}
}