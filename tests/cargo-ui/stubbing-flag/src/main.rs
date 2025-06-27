// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This tests that enabling stubbing and using `--harness` arguments flow from
//! `kani-driver` to `kani-compiler`.

#[kani::proof]
fn main() {}
