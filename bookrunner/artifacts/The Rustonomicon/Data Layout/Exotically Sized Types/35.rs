// compile-flags: --edition 2018
#![allow(unused)]
// Can't be stored on the stack directly
fn main() {
struct MySuperSlice {
    info: u32,
    data: [u8],
}
}