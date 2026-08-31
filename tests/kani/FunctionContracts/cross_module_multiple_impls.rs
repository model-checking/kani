// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// Check that Kani can verify contracts on methods where the base type has multiple
// same-named methods across impl blocks that live OUTSIDE the type's home module. This
// extends the same-module case in multiple_inherent_impls.rs (c.f.
// https://github.com/model-checking/kani/issues/3773) to the `<impl path::Type<Args>>`
// path form def_path_str renders when an impl's module differs from its self type's.
// One candidate's generic argument is a tuple type, so this also exercises the
// tuple-vs-trait-object-bound paren distinction in that path's disambiguation.
pub mod ty {
    pub struct S<T>(pub T);
}

pub mod ops {
    use crate::ty::S;

    impl S<(u32, u64)> {
        #[kani::requires(self.0.0.checked_mul(2).is_some() && self.0.1.checked_mul(2).is_some())]
        pub fn double(self) -> (u32, u64) {
            (self.0.0 * 2, self.0.1 * 2)
        }
    }

    impl S<u64> {
        #[kani::requires(self.0.checked_mul(2).is_some())]
        pub fn double(self) -> u64 {
            self.0 * 2
        }
    }
}

mod verify {
    use crate::ty::S;

    #[kani::proof_for_contract(S::<(u32, u64)>::double)]
    fn verify_double_tuple_arg() {
        let x: S<(u32, u64)> = S((2, 3));
        x.double();
    }

    #[kani::proof_for_contract(S::<u64>::double)]
    fn verify_double_u64() {
        let x: S<u64> = S(2);
        x.double();
    }
}
