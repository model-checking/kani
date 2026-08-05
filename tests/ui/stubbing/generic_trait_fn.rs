// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness harness -Z stubbing
// This stub should work, since the function signatures are identical,
// but we do not yet support stubbing/contracts on trait fns with generic arguments
// c.f. https://github.com/model-checking/kani/issues/1997#issuecomment-3134614734.
// For now, test that we emit a nice error message.

trait TraitX {
    fn generic_fn<U: Clone>(&self, x: U) -> U;
}

trait TraitY {
    fn generic_fn<U: Clone>(&self, x: U) -> U;
}

struct TestStruct;

impl TraitX for TestStruct {
    fn generic_fn<U: Clone>(&self, x: U) -> U {
        x
    }
}

impl TraitY for TestStruct {
    fn generic_fn<U: Clone>(&self, x: U) -> U {
        x
    }
}

#[kani::proof]
#[kani::stub(<TestStruct as TraitX>::generic_fn, <TestStruct as TraitY>::generic_fn)]
fn unsupported_harness() {}
