// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness mod2::harness -Z stubbing
//
//! This tests whether we correctly find harnesses during stubbing that are
//! specified with a partially qualified name.

mod mod1 {
    mod mod2 {
        fn foo() -> u32 {
            0
        }

        fn bar() -> u32 {
            42
        }

        #[kani::proof]
        #[kani::stub(foo, bar)]
        fn harness() {
            assert_eq!(foo(), 42);
        }
    }
}
