// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --fail-fast -Z unstable-options --jobs 4 --output-format=terse

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

    #[kani::proof]
    fn test_05_fail() {
        assert!(false, "Fifth failure");
    }

    #[kani::proof]
    fn test_06_fail() {
        assert!(false, "Sixth failure");
    }

    #[kani::proof]
    fn test_07_fail() {
        assert!(false, "Seventh failure");
    }

    #[kani::proof]
    fn test_08_fail() {
        assert!(false, "Eighth failure");
    }

    #[kani::proof]
    fn test_09_fail() {
        assert!(false, "Ninth failure");
    }

    #[kani::proof]
    fn test_10_fail() {
        assert!(false, "Tenth failure");
    }
}