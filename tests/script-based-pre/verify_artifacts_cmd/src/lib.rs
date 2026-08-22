// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fixture for the verify-artifacts integration test. Two harnesses: one that
//! verifies, one that fails — proving the verdict is honest, not vacuous.

#[cfg(kani)]
mod proofs {
    #[kani::proof]
    fn check_add_identity() {
        let x: u8 = kani::any();
        assert_eq!(x.wrapping_add(0), x);
    }

    #[kani::proof]
    fn check_intentional_failure() {
        let x: u8 = kani::any();
        // Fails for x == 255. The deliberate failure proves the pipeline
        // reaches CBMC and reads the verdict — not merely that the command
        // exited without crashing.
        assert!(x < 255);
    }
}
