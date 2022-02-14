// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --output-format old

// kani-flags: --dry-run --object-bits 10
// cbmc-flags: --object-bits 8

// `--dry-run` causes Kani to print out commands instead of running them
// In `expected` you will find substrings of these commands because the
// concrete paths depend on your working directory.
fn main() {}
