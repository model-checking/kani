// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This is just to test that we can call an unstable feature.
//! We also ensure that non-reachable unstable features do not affect our analysis.

#[kani::proof]
pub fn harness() {
    defs::no_op();
}
