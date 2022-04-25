// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --dry-run --function main

// `--dry-run` causes Kani to print out commands instead of running them
// In `expected` you will find substrings of these commands because the
// concrete paths depend on your working directory.
#[kani::proof]
fn main() {}
