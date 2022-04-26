// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --enable-unstable --dry-run --function main
// cbmc-flags: --function main

//! This testcase is to ensure that user cannot pass --function as cbmc-flags
//! with our driver logic.

/// This shouldn't run.
#[kani::proof]
fn main() {}
