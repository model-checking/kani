// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Check that users can use any Option, Result and array if they implement Arbitrary type.
//! Note: This test could use some clean up. It uses macros and PercentTraint because it used to
//! check the Invariant trait as well but we have removed that.
extern crate kani;

use kani::Arbitrary;

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

percent_type!(PercentArbitrary);

impl Arbitrary for PercentArbitrary {
    fn any() -> Self {
        let val = kani::any();
        kani::assume(val <= 100);
        PercentArbitrary { inner: val }
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
fn check_arbitrary() {
    check::<PercentArbitrary>();
    check_result::<PercentArbitrary>();
    check_array::<PercentArbitrary>();
}
