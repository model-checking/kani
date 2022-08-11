// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that users can use any Option, Result and array if they implement Arbitrary or Invariant.
#![cfg_attr(kani, feature(min_specialization))]

extern crate kani;
use kani::Arbitrary;
use kani::Invariant;

trait PercentTrait {
    fn val(&self) -> u8;
    fn ok(&self) -> bool;
}

macro_rules! percent_type {
    ( $type: tt ) => {
        struct $type {
            inner: u8,
        }
        impl PercentTrait for $type {
            fn val(&self) -> u8 {
                self.inner
            }

            fn ok(&self) -> bool {
                self.inner <= 100
            }
        }
    };
}

percent_type!(Percent);
percent_type!(PercentInvariant);
percent_type!(PercentArbitrary);

unsafe impl Invariant for PercentInvariant {
    fn is_valid(&self) -> bool {
        self.ok()
    }
}

impl Arbitrary for PercentArbitrary {
    fn any() -> Self {
        let val = kani::any();
        kani::assume(val <= 100);
        PercentArbitrary { inner: val }
    }
}

unsafe impl Invariant for Percent {
    fn is_valid(&self) -> bool {
        self.ok()
    }
}

impl Arbitrary for Percent {
    fn any() -> Self {
        let val = kani::any();
        kani::assume(val <= 100);
        Percent { inner: val }
    }
}

fn check<T: PercentTrait + Arbitrary>() {
    let var = Option::<T>::any();
    match var {
        None => assert!(T::any().ok()),
        Some(p) => assert!(p.ok()),
    }
}

fn check_result<T: PercentTrait + Arbitrary>() {
    let var = Result::<T, ()>::any();
    match var {
        Err(_) => assert!(T::any().ok()),
        Ok(p) => assert!(p.ok()),
    }
}

fn check_array<T: PercentTrait + Arbitrary>()
where
    [T; 10]: Arbitrary,
{
    let var: [T; 10] = kani::any();
    assert!(var.iter().all(|e| e.ok()));
}

#[kani::proof]
#[kani::unwind(12)]
fn check_invariant() {
    check::<PercentInvariant>();
    check_result::<PercentInvariant>();
    check_array::<PercentInvariant>();
}

#[kani::proof]
#[kani::unwind(12)]
fn check_arbitrary() {
    check::<PercentArbitrary>();
    check_result::<PercentArbitrary>();
    check_array::<PercentArbitrary>();
}

#[kani::proof]
#[kani::unwind(12)]
fn check_both() {
    check::<Percent>();
    check_result::<Percent>();
    check_array::<Percent>();
}
