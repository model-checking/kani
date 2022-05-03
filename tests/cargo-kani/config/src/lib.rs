// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This testcase requires an unwind threshold of less than 10 and no-unwind-checks in order to
//! succeed. These parameters are set inside Cargo.toml.

#[cfg(kani)]
mod kani_tests {
    #[kani::proof]
    fn check_config() {
        let mut counter = 0;
        while kani::any() {
            counter += 1;
            assert!(counter < 10, "Cargo.toml should've configure kani to stop at iteration 5");
        }
    }
}
