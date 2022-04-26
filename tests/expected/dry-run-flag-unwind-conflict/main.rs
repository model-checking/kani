// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --dry-run --default-unwind 2 --enable-unstable --function main --cbmc-args --unwindset 2

// `--dry-run` causes Kani to print out commands instead of running them
// In `expected` you will find substrings of these commands because the
// concrete paths depend on your working directory.
#[kani::proof]
fn main() {}
