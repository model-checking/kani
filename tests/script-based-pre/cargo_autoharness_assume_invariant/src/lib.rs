// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the autoharness subcommand assumes the safety invariant (`kani::Invariant`)
// of the nondeterministic values it generates, including for nested types,
// regardless of how the type obtains its `Arbitrary` implementation
// (user-provided, derived, or synthesized by the compiler).
// Note that in the "TEST NOTE" comments below, "synthesized `Arbitrary`" refers to the
// implementation that the compiler generates for types that do not implement `Arbitrary`
// in source code (c.f. the `AutomaticArbitraryPass` compiler pass).

use kani::Invariant;

// TEST NOTE: manual `Invariant` impl and no `Arbitrary` impl (the compiler synthesizes one).
// The automatic harness for `manual_invariant_synthesized_arbitrary` should respect the invariant.
pub struct PercentManual {
    val: u8,
}

impl kani::Invariant for PercentManual {
    fn is_safe(&self) -> bool {
        self.val <= 100
    }
}

pub fn manual_invariant_synthesized_arbitrary(p: PercentManual) {
    assert!(p.val <= 100);
}

// TEST NOTE: unconstrained derived `Arbitrary` and a manual `Invariant` impl.
// The automatic harness for `manual_invariant_derived_arbitrary` should respect the invariant.
#[derive(kani::Arbitrary)]
pub struct PercentDerived {
    val: u8,
}

impl kani::Invariant for PercentDerived {
    fn is_safe(&self) -> bool {
        self.val <= 100
    }
}

pub fn manual_invariant_derived_arbitrary(p: PercentDerived) {
    assert!(p.val <= 100);
}

// TEST NOTE: derived `Invariant` via `#[safety_constraint]`, which the derived `Arbitrary`
// also assumes (existing behavior that should keep working).
#[derive(kani::Arbitrary, kani::Invariant)]
pub struct PercentConstrained {
    #[safety_constraint(*val <= 100)]
    val: u8,
}

pub fn constrained_arbitrary(p: PercentConstrained) {
    assert!(p.is_safe());
}

// TEST NOTE: nested invariants. `Outer` derives an unconstrained `Arbitrary` and an
// `Invariant` impl whose `is_safe` is the conjunction of its fields' `is_safe` methods.
// The automatic harness for `nested_invariant` should respect the inner invariants.
#[derive(kani::Arbitrary, kani::Invariant)]
pub struct Outer {
    manual: PercentDerived,
    constrained: PercentConstrained,
}

pub fn nested_invariant(o: Outer) {
    assert!(o.manual.val <= 100 && o.constrained.val <= 100);
}

// TEST NOTE: nested invariant inside a type that implements neither `Arbitrary` nor
// `Invariant`. The synthesized `Arbitrary` for `Wrapper` should still respect the
// invariant of its field.
pub struct Wrapper {
    inner: PercentManual,
}

pub fn nested_invariant_synthesized_arbitrary(w: Wrapper) {
    assert!(w.inner.val <= 100);
}

// TEST NOTE: invariants are assumed for values behind references too.
pub fn invariant_behind_ref(p: &PercentManual) {
    assert!(p.val <= 100);
}

// TEST NOTE: negative control; the harness must not assume anything beyond the invariant,
// so this function should fail verification (`val` may be anywhere in 0..=100).
pub fn over_assume_negative_control(p: PercentManual) {
    assert!(p.val <= 50);
}

// TEST NOTE: negative control; `Unconstrained` has no `Invariant` impl, so the harness
// must not invent one, and this function should fail verification.
pub struct Unconstrained {
    val: u8,
}

pub fn no_invariant_negative_control(u: Unconstrained) {
    assert!(u.val <= 100);
}
