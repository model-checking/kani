// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// From https://github.com/model-checking/kani/issues/3101

pub const SOME_CONSTANT: u32 = 42;

pub struct SomeStruct {
    pub some_field: u32,
}

#[cfg(kani)]
impl kani::Arbitrary for SomeStruct {
    fn any() -> Self {
        SomeStruct { some_field: kani::any() }
    }
}

#[cfg(kani)]
mod verification {
    use super::*;

    #[kani::proof]
    fn zero() {
        assert_ne!(SOME_CONSTANT, 0);
    }
}
