// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z function-contracts -Z stubbing
//
//! Test that the `stub_verified` / `Arbitrary` cycle check does not fire when the
//! return type's `Arbitrary` implementation calls a *different* monomorphization
//! of the same generic stubbed function.
//!
//! `<Cyclic as Arbitrary>::any` calls `clamp::<Safe>`, not the `clamp::<Cyclic>`
//! instance under check. `Safe` derives `Arbitrary`, so the chain terminates and
//! there is no recursion. Comparing callees to the target by `DefId` instead of
//! by monomorphized instance would reject this working proof.
//!
//! This is the counterpart to
//! `tests/expected/function-contract/stub_verified_arbitrary_cycle_generic.rs`,
//! where the `Arbitrary` impl does call the same instantiation and Kani errors.
//!
//! See https://github.com/model-checking/kani/pull/4571

trait Clampable: Copy {
    fn val(self) -> u64;
    fn make(v: u64) -> Self;
}

#[derive(Clone, Copy, kani::Arbitrary)]
struct Safe {
    v: u64,
}

impl Clampable for Safe {
    fn val(self) -> u64 {
        self.v
    }
    fn make(v: u64) -> Self {
        Safe { v }
    }
}

#[derive(Clone, Copy)]
struct Cyclic {
    v: u64,
}

impl Clampable for Cyclic {
    fn val(self) -> u64 {
        self.v
    }
    fn make(v: u64) -> Self {
        Cyclic { v }
    }
}

// Calls `clamp::<Safe>`, a different instantiation than the one being stubbed.
impl kani::Arbitrary for Cyclic {
    fn any() -> Self {
        let s = clamp(Safe { v: kani::any() });
        Cyclic { v: s.val() }
    }
}

#[kani::ensures(|r: &T| r.val() <= 10)]
fn clamp<T: Clampable>(x: T) -> T {
    if x.val() > 10 { T::make(10) } else { x }
}

#[kani::proof_for_contract(clamp)]
fn check_clamp_contract() {
    clamp(Safe { v: kani::any() });
}

#[kani::proof]
#[kani::stub_verified(clamp)]
fn check_other_instantiation() {
    let c: Cyclic = kani::any();
    assert!(clamp(c).val() <= 10);
}
