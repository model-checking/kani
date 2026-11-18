// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness harness -Z stubbing
// Resolve method-level arguments on both sides of a trait-method stub.

trait TraitX {
    fn generic_fn<U: Clone>(&self, x: U) -> U;
}

trait TraitY {
    fn generic_fn<U: Clone>(&self, x: U) -> U;
}

struct TestStruct;

impl TraitX for TestStruct {
    fn generic_fn<U: Clone>(&self, _: U) -> U {
        unreachable!("original method must be stubbed")
    }
}

impl TraitY for TestStruct {
    fn generic_fn<U: Clone>(&self, x: U) -> U {
        x
    }
}

#[kani::proof]
#[kani::stub(<TestStruct as TraitX>::generic_fn, <TestStruct as TraitY>::generic_fn)]
fn harness() {
    assert_eq!(<TestStruct as TraitX>::generic_fn(&TestStruct, 3u8), 3);
    assert_eq!(<TestStruct as TraitX>::generic_fn(&TestStruct, 5u64), 5);
}
