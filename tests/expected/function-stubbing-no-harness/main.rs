// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness foo --enable-unstable --enable-stubbing
//
//! This tests whether we detect missing harnesses during stubbing.

#[kani::proof]
fn main() {}
