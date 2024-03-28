// compile-flags: --edition 2021
#![allow(unused)]
pub use self::implementation::api;

mod implementation {
    pub mod api {
        pub fn f() {}
    }
}

fn main() {}