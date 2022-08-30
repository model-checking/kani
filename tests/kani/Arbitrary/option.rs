// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that the Arbitrary implementation for Option respect underlying type invariants.

extern crate kani;

struct MyType {
    pub val: u8,
}

impl kani::Arbitrary for MyType {
    fn any() -> Self {
        let val = kani::any();
        kani::assume(val < 100);
        MyType { val }
    }
}

#[kani::proof]
fn main() {
    let option: Option<MyType> = kani::any();
    match option {
        Some(v) => assert!(v.val < 100),
        None => (),
    }
}
