// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! This tests whether we detect syntactically misformed `kani::stub` annotations.

#[kani::proof]
#[kani::stub(foo)]
#[kani::stub(foo, 42)]
#[kani::stub("foo", bar)]
#[kani::stub(foo, bar, baz)]
fn main() {}
