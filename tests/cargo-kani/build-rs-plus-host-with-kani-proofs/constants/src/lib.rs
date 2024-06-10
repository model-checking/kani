// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// From https://github.com/model-checking/kani/issues/3101

#[cfg(not(any(kani, kani_host)))]
pub const SOME_CONSTANT: u32 = 0;
#[cfg(kani)]
pub const SOME_CONSTANT: u32 = 1;
#[cfg(kani_host)]
pub const SOME_CONSTANT: u32 = 2;

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
    fn one() {
        assert_eq!(constants::SOME_CONSTANT, 1);
    }
}
