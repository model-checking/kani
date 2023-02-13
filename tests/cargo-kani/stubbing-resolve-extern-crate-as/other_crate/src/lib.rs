// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub fn magic_number13() -> u32 {
    13
}

pub mod inner_mod {
    pub fn magic_number42() -> u32 {
        42
    }
}

pub struct MyType {}

impl MyType {
    pub fn magic_number101() -> u32 {
        101
    }
}
