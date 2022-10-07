// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::*;

// This test invokes `cago kani assess`, so this test gets picked up
// and run under kani as if it were a proof harness.
// Here we try to use a dependency (anyhow).

// At time of writing this actually leads to an unwinding assertion failure,
// but all we're really looking for in `expected` is that assess ran.

#[test]
fn a_test_using_anyhow() -> Result<()> {
    Ok(()).context("words")
}
