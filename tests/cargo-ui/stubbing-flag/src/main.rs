// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This tests that the `--enable-stubbing` and `--harness` arguments flow from `kani-driver` to `kani-compiler`
//! (and that we warn that no actual stubbing is being performed).

#[kani::proof]
fn main() {}
