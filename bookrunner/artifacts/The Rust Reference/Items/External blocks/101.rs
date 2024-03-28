// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
extern "C" {
    fn foo(x: i32, ...);
    fn with_name(format: *const u8, args: ...);
}
}