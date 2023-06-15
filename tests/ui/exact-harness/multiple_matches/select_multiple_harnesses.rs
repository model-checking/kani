// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --harness first::check_blah --exact --harness second::verify_foo
//! Ensure that only the specified harnesses are run

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

mod second {
    #[kani::proof]
    fn verify_foo() {
        assert!(1 == 1);
    }

    #[kani::proof]
    fn verify_blah() {
        assert!(2 == 2);
    }

    #[kani::proof]
    fn verify_harness() {
        assert!(3 == 3);
    }
}
