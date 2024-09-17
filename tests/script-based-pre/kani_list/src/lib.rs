// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This test replicates the module structure from the running example in the list RFC.
//! It ensures that the list command across modules, and with modifies clauses, history expressions, and generic functions.

mod example {
    pub mod implementation {
        #[kani::requires(*x < 4)]
        #[kani::requires(*x > 2)]
        #[kani::ensures(|_| old(*x - 1) == *x)]
        #[kani::ensures(|_| *x == 4)]
        #[kani::modifies(x)]
        pub fn bar(x: &mut u32) {
            *x += 1;
        }

        #[kani::requires(true)]
        #[kani::ensures(|_| old(*x) == *x)]
        pub fn foo<T: Copy + std::cmp::PartialEq>(x: &mut T) -> T {
            *x
        }

        #[kani::requires(*x < 100)]
        #[kani::modifies(x)]
        pub fn func(x: &mut i32) {
            *x *= 1;
        }

        pub fn baz(x: &mut i32) {
            *x /= 1;
        }
    }

    mod prep {
        #[kani::requires(s.len() < 10)]
        fn parse(s: &str) -> u32 {
            s.parse().unwrap()
        }
    }

    mod verify {
        use crate::example::implementation;

        #[kani::proof_for_contract(implementation::bar)]
        fn check_bar() {
            let mut x = 7;
            implementation::bar(&mut x);
        }

        #[kani::proof_for_contract(implementation::foo)]
        fn check_foo_u32() {
            let mut x: u32 = 7;
            implementation::foo(&mut x);
        }

        #[kani::proof_for_contract(implementation::foo)]
        fn check_foo_u64() {
            let mut x: u64 = 7;
            implementation::foo(&mut x);
        }

        #[kani::proof_for_contract(implementation::func)]
        fn check_func() {
            let mut x = 7;
            implementation::func(&mut x);
        }

        #[kani::proof_for_contract(implementation::baz)]
        fn check_baz() {
            let mut x = 7;
            implementation::baz(&mut x);
        }

        #[kani::proof]
        fn check_modify() {}

        #[kani::proof]
        fn check_new() {}
    }
}
