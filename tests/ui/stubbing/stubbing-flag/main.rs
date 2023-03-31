// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness main --enable-unstable --enable-stubbing
//
//! This tests that the `--enable-stubbing` and `--harness` arguments flow from `kani-driver` to `kani-compiler`.

#[kani::proof]
fn main() {}
