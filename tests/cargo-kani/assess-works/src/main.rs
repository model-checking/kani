// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::*;

#[test]
fn a_test_using_anyhow() -> Result<()> {
    Ok(()).context("words")
}
