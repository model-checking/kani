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

    fn foo(&self) -> u32 {
        0
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
fn harness() {
    let x = X::new();
    assert_eq!(x.foo(), 0);
    assert_eq!(A::bar(&x), 200);
    assert_eq!(<X as B>::bar(&x), 300);
}

#[kani::proof]
#[kani::stub(X::foo, <X as A>::bar)]
#[kani::stub(<X as A>::bar, <X as B>::bar)]
fn stubbed_harness() {
    let x = X::new();
    assert_eq!(x.foo(), 300);
    assert_eq!(A::bar(&x), 300);
    assert_eq!(<X as B>::bar(&x), 300);
}
