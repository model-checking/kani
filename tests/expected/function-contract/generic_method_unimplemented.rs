// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// A `proof_for_contract` on a trait method that carries its OWN generic parameter
// (`fn compute<T>`) is the case kani#1997 still does not support. Instantiating the
// path's generic arguments (this change) binds the self type and the trait's arguments,
// not a method-level type parameter, so this must still fail to resolve.

struct S(u32);

trait Compute {
    fn compute<T>(&self, x: T) -> u32;
}

impl Compute for S {
    #[kani::requires(self.0 < 100)]
    #[kani::ensures(|r| *r == self.0)]
    fn compute<T>(&self, _x: T) -> u32 {
        self.0
    }
}

#[cfg(kani)]
mod verify {
    use super::*;

    #[kani::proof_for_contract(<S as Compute>::compute)]
    fn check_generic_method() {
        let s = S(kani::any());
        let _ = s.compute(0u8);
    }
}
