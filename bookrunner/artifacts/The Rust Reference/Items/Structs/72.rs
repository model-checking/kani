// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
struct Cookie {}
const Cookie: Cookie = Cookie {};
let c = [Cookie, Cookie {}, Cookie, Cookie {}];
}