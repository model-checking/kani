// compile-flags: --edition 2021
#![allow(unused)]
// Fixed size
fn main() {
let arr = [1, 2, 3];
match arr {
    [1, _, _] => "starts with one",
    [a, b, c] => "starts with something else",
};
}