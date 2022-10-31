// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness main --enable-unstable --enable-stubbing
//
//! This tests that the --enable-stubbing and --harness flags flow from Kani driver to Kani compiler
//! (and that we warn that no actual stubbing is being performed).

fn foo() {}

fn bar() {}

#[kani::proof]
fn main() {}
