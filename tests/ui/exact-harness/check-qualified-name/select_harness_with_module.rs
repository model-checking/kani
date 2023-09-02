// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --harness first::check_foo --exact
//! Ensure that only the specified harness is run

mod first {
    #[kani::proof]
    fn check_foo() {
        assert!(1 == 1);
    }

    /// A harness that will fail verification if it is run.
    #[kani::proof]
    fn check_blah() {
        assert!(1 == 2);
    }

    /// A harness that will fail verification if it is run.
    #[kani::proof]
    fn ignore_third_harness() {
        assert!(3 == 2);
    }
}
