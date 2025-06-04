// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z stubbing
//! Check that Kani supports stubbing methods in trait implementations
//! using the motivating example from https://github.com/model-checking/kani/issues/1997

trait A {
    fn foo(&self) -> u32;

    fn bar(&self) -> u32;
}

trait B {
    fn bar(&self) -> u32;
}

struct X {}

impl X {
    fn new() -> Self {
        Self {}
    }
}

impl A for X {
    fn foo(&self) -> u32 {
        100
    }

    fn bar(&self) -> u32 {
        200
    }
}

impl B for X {
    fn bar(&self) -> u32 {
        300
    }
}

#[kani::proof]
// A::baz doesn't exist
#[kani::stub(<X as B>::bar, A::baz)]
// X::bar is ambiguous; two impls could match
#[kani::stub(X::bar, A::foo)]
// B::bar doesn't have a body
#[kani::stub(X::foo, B::bar)]
fn resolution_errors() {}
