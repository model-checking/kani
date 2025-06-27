// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This is just to test that compilation fais if we don't enable all unstable features that
//! are reachable from a harness to be verified.

#[kani::proof]
pub fn harness() {
    defs::no_op();
    defs::always_fails();
}
