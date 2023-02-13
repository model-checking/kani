// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub use my_mod1::*;
pub use my_mod2 as inner_mod;
pub use my_mod3::MyType;

mod my_mod1 {
    pub fn magic_number13() -> u32 {
        13
    }
}

pub mod my_mod2 {
    pub fn magic_number42() -> u32 {
        42
    }
}

mod my_mod3 {
    pub struct MyType {}

    impl MyType {
        pub fn magic_number101() -> u32 {
            101
        }
    }
}
