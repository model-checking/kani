// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness main -Z stubbing
//
//! This tests that enabling stubbing and `--harness` argument flow from `kani-driver` to `kani-compiler`.

#[kani::proof]
fn main() {}
