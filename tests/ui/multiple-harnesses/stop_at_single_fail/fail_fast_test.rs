// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z unstable-options --jobs 4 --output-format=terse

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
        assert!(false, "Should not run - third");
    }

    #[kani::proof]
    fn test_04_fail() {
        assert!(false, "Should not run - fourth");
    }

    #[kani::proof]
    fn test_05_fail() {
        assert!(false, "Should not run - fifth");
    }

    #[kani::proof]
    fn test_06_fail() {
        assert!(false, "Should not run - sixth");
    }

    #[kani::proof]
    fn test_07_fail() {
        assert!(false, "Should not run - seventh");
    }

    #[kani::proof]
    fn test_08_fail() {
        assert!(false, "Should not run - eighth");
    }

    #[kani::proof]
    fn test_09_fail() {
        assert!(false, "Should not run - ninth");
    }

    #[kani::proof]
    fn test_10_fail() {
        assert!(false, "Should not run - tenth");
    }
}
