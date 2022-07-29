// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
#[export_name = "exported_symbol_name"]
pub fn name_in_rust() { }
}