// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn foo() {}

fn bar() {}

#[kani::proof]
#[kani::stub(foo, bar)]
fn main() {}
