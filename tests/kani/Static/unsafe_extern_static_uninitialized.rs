// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-verify-fail

//! This test exercises the `is_extern` property that CBMC makes use of during proof initialization.
//! CBMC has two behaviors with an *uninitialized* static variable:
//!   1. If it is declared `extern`, then it is nondet-initialized. (Possible in unsafe Rust)
//!   2. If it is not `extern`, then it is zero-initialized. (Not possible in Rust)
//!
//! Here we test to see that we observe the nondet-initialization.
//! If this extern static were zero-initialized, the assert below would pass.
//! Instead, we expect to see failure, because nondet-initialization could be 1.

extern "C" {
    static an_uninitialized_variable: u32;
}

#[kani::proof]
fn check_extern_static_isnt_deterministic() {
    // If this is zero-initialized, this assertion will pass, but this
    // test is labeled 'kani-verify-fail', and so the test would fail.
    assert!(unsafe { an_uninitialized_variable } != 1);
}
