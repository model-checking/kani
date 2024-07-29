// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that the `#[safety_constraint(...)]` attribute is picked up when
//! deriving the `Arbitrary` and `Invariant` implementations.

//! In this case, we test the attribute on a struct with a generic type `T`
//! which requires the bound `From<i32>` because of the comparisons in the
//! `#[safety_constraint(...)]` predicate. The struct represents an abstract
//! value for which we only track its sign. The actual value is kept private.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
#[safety_constraint((*positive && *conc_value >= 0.into()) || (!*positive && *conc_value < 0.into()))]
struct AbstractValue<T>
where
    T: PartialOrd + From<i32>,
{
    pub positive: bool,
    conc_value: T,
}

#[kani::proof]
fn check_abstract_value() {
    let value: AbstractValue<i32> = kani::any();
    assert!(value.is_safe());
}
