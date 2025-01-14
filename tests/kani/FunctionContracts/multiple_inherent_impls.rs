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
