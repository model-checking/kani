// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --harness ignore_third_harness --exact
//! Check that we just ignore non-matching filters

mod first {
    #[kani::proof]
    fn check_foo() {
        assert!(1 == 1);
    }

    #[kani::proof]
    fn check_blah() {
        assert!(2 == 2);
    }

    /// A harness that will fail verification if it is run.
    #[kani::proof]
    fn ignore_third_harness() {
        assert!(3 == 2);
    }
}
