// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --fail-fast
//! Ensure that the verification process stops as soon as one of the harnesses fails.

mod tests {
    #[kani::proof]
    fn test_01_fail() {
        assert!(false, "First failure");
    }

    #[kani::proof]
    fn test_02_fail() {
        assert!(false, "Second failure");
    }

    #[kani::proof]
    fn test_03_fail() {
        assert!(false, "Third failure");
    }

    #[kani::proof]
    fn test_04_fail() {
        assert!(false, "Fourth failure");
    }
}