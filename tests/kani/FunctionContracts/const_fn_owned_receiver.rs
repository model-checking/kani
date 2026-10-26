// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts -Zstubbing

//! A contract on a `const fn` that takes `self` by value must compile and
//! verify when `Self` has a destructor. The replace stub used to leave `self`
//! to drop in the annotated function's own scope, which `const fn`s reject
//! (E0493); the other arguments are already covered by the #3667 reassignments.
//!
//! `Owned` has an explicit `Drop` (so it is a needs-drop receiver, like a field
//! of `Box`), but wraps a `u32` so the replace-mode harness can havoc it. The
//! body moves `self` into the return without dropping it, so it is itself
//! const-legal; the only drop of `self` previously came from the contract.

struct Owned(u32);

impl Drop for Owned {
    fn drop(&mut self) {}
}

impl kani::Arbitrary for Owned {
    fn any() -> Self {
        Owned(kani::any())
    }
}

impl Owned {
    #[kani::requires(self.0 < 100)]
    #[kani::ensures(|result: &Owned| result.0 == old(self.0))]
    const fn identity(self) -> Owned {
        self
    }
}

/// Check mode: compiles the full contract expansion (including the replace arm
/// that previously hit E0493) and verifies the contract against the body.
#[kani::proof_for_contract(Owned::identity)]
fn check_owned_receiver_const_fn() {
    let out = Owned(kani::any_where(|v: &u32| *v < 100)).identity();
    assert!(out.0 < 100);
}

fn caller(o: Owned) -> Owned {
    o.identity()
}

/// Replace mode: stubs `identity` with its contract, exercising the replace
/// closure (with the tail receiver-move) end to end.
#[kani::proof]
#[kani::stub_verified(Owned::identity)]
fn check_owned_receiver_replace() {
    let out = caller(Owned(kani::any_where(|v: &u32| *v < 100)));
    assert!(out.0 < 100);
}
