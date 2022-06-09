// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test's purpose is to make use of a dev-dependency.
//! We previously had issues with dev-dependencies not being
//! resolvable because we did not produce 'rlib' for them,
//! and tests are ultimately `crate-type=bin`

// Need cfg(test) too for now. TODO: https://github.com/model-checking/kani/issues/1258
#[cfg(all(kani, test))]
mod proofs {
    use anyhow; // trigger dependency resolution

    #[kani::proof]
    fn check_trivial() {
        assert!(1 == 1);
    }
}
