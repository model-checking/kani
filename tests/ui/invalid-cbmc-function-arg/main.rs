// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z unstable-options
// cbmc-flags: --function main

//! This testcase is to ensure that user cannot pass --function as cbmc-flags
//! with our driver logic.

/// This shouldn't run.
#[kani::proof]
fn main() {}
