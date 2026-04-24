// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! Test stubbing a trait default method that IS overridden in the impl.
//! The stub should replace the overridden implementation, not the default.

trait HasDefault {
    fn method(&self) -> u32 {
        0
    }
}

struct MyType;

impl HasDefault for MyType {
    fn method(&self) -> u32 {
        100
    }
}

fn stub_method(_x: &MyType) -> u32 {
    42
}

#[kani::proof]
#[kani::stub(<MyType as HasDefault>::method, stub_method)]
fn check_stub_overridden_default() {
    let x = MyType;
    assert_eq!(x.method(), 42);
}
