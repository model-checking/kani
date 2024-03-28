// compile-flags: --edition 2018
#![allow(unused)]
#[no_mangle]
pub extern "C" fn hello_from_rust() {
    println!("Hello from Rust!");
}
fn main() {}