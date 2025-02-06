// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z unstable-options --jobs 1 --output-format=terse

mod tests {
    // Quick tests that should pass immediately
    #[kani::proof]
    fn test_quick_pass1() {
        assert!(true, "Quick pass 1");
    }

    #[kani::proof]
    fn test_quick_pass2() {
        assert!(true, "Quick pass 2");
    }

    // A test that will fail quickly
    #[kani::proof]
    fn test_quick_fail() {
        assert!(false, "Quick fail that should stop others");
    }

    // A slow test that might be in progress when failure is detected
    #[kani::proof]
    fn test_slow_pass() {
        let mut sum = 0;
        for i in 0..10 {
            if kani::any() {
                sum += i;
            }
        }
        assert!(sum >= 0, "Slow pass");
    }

    // Another slow test with failure
    #[kani::proof]
    fn test_slow_fail() {
        let mut product = 1;
        for i in 0..10 {
            if kani::any() {
                product *= i;
            }
        }
        assert!(product == 1, "Slow fail");
    }

    // More quick tests that should be prevented from starting
    #[kani::proof]
    fn test_should_skip1() {
        assert!(true, "Should skip 1");
    }

    #[kani::proof]
    fn test_should_skip2() {
        assert!(true, "Should skip 2");
    }

    #[kani::proof]
    fn test_should_skip3() {
        assert!(true, "Should skip 3");
    }
}
