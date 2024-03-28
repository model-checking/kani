// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
extern {
    #[link_name = "actual_symbol_name"]
    fn name_in_rust();
}
}