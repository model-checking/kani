// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This testcase requires an unwind threshold of less than 10 and no-unwind-checks in order to
//! succeed. These parameters are set inside Cargo.toml.

#[cfg(rmc)]
mod rmc_tests {
    #[rmc::proof]
    fn check_config() {
        let mut counter = 0;
        while rmc::any() {
            counter += 1;
            assert!(counter < 10, "Cargo.toml should've configure rmc to stop at iteration 5");
        }
    }
}
