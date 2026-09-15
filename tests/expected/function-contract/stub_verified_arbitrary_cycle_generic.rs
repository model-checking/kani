// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts -Zstubbing

//! Test that the `stub_verified` / `Arbitrary` cycle check inspects every
//! monomorphization of a generic target, not just the first one.
//!
//! `clamp` is instantiated at both `Safe` and `Cyclic`. Only `Cyclic` has an
//! `Arbitrary` implementation that calls back into `clamp`, and `clamp::<Safe>`
//! is transformed first. Deduplicating the check per `FnDef` rather than per
//! monomorphized instance would let the acyclic `Safe` instantiation consume the
//! only slot, skipping `Cyclic` entirely and reintroducing the hang.
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

// This `Arbitrary` implementation routes back into the generic stubbed function.
impl kani::Arbitrary for Cyclic {
    fn any() -> Self {
        clamp(Cyclic { v: kani::any() })
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

// The acyclic `Safe` instantiation is used before the cyclic `Cyclic` one.
#[kani::proof]
#[kani::stub_verified(clamp)]
fn check_both_instantiations() {
    let s: Safe = kani::any();
    assert!(clamp(s).val() <= 10);
    let c: Cyclic = kani::any();
    assert!(clamp(c).val() <= 10);
}
