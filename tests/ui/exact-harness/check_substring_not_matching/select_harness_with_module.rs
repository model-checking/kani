// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --harness first::harness --exact
//! Ensure that only the harness specified with --exact is picked up

mod first {
    #[kani::proof]
    fn harness() {
        assert!(1 == 1);
    }

    /// A harness that will fail verification if it is picked up.
    #[kani::proof]
    fn harness_1() {
        assert!(1 == 2);
    }

    /// A harness that will fail verification if it is picked up.
    #[kani::proof]
    fn harness_2() {
        assert!(3 == 2);
    }
}
