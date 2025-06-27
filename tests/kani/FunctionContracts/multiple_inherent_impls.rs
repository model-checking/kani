// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// Check that Kani can verify contracts on methods where the base type has multiple inherent impls,
// c.f. https://github.com/model-checking/kani/issues/3773

struct NonZero<T>(T);

impl NonZero<u32> {
    #[kani::requires(self.0.checked_mul(x).is_some())]
    fn unchecked_mul(self, x: u32) -> u32 {
        self.0 * x
    }
}

impl NonZero<i32> {
    #[kani::requires(self.0.checked_mul(x).is_some())]
    fn unchecked_mul(self, x: i32) -> i32 {
        self.0 * x
    }
}

#[kani::proof_for_contract(NonZero::<i32>::unchecked_mul)]
fn verify_unchecked_mul_ambiguous_path() {
    let x: NonZero<i32> = NonZero(-1);
    x.unchecked_mul(-2);
}

// Test that the resolution still works if the function in question is nested inside multiple modules,
// i.e. the absolute path to the function can be arbitrarily long.
// As long as the generic arguments and function name match, resolution should succeed.
// This mimics the actual structure of NonZero relative to its harnesses in the standard library.
pub mod num {
    pub mod negative {
        pub struct NegativeNumber<T>(pub T);

        impl NegativeNumber<i32> {
            #[kani::requires(self.0.checked_mul(x).is_some())]
            pub fn unchecked_mul(self, x: i32) -> i32 {
                self.0 * x
            }
        }

        impl NegativeNumber<i16> {
            #[kani::requires(self.0.checked_mul(x).is_some())]
            pub fn unchecked_mul(self, x: i16) -> i16 {
                self.0 * x
            }
        }
    }
}

mod verify {
    use crate::num::negative::*;

    #[kani::proof_for_contract(NegativeNumber::<i32>::unchecked_mul)]
    fn verify_unchecked_mul_ambiguous_path() {
        let x: NegativeNumber<i32> = NegativeNumber(-1);
        x.unchecked_mul(-2);
    }
}
