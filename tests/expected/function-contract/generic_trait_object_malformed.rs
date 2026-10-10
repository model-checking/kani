// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// Malformed trait-object arguments the `Type::TraitObject` arm declines to build:
// each must fail to resolve cleanly (no ICE, no wrong impl), the same way the
// associated-type cases do (generic_trait_object_binding.rs,
// generic_trait_object_unspecified_assoc.rs). Supported cases live in
// tests/kani/FunctionContracts/generic_trait_object_argument.rs.
//   * a marker-only object with no principal trait (`dyn Send`),
//   * two principal traits (`dyn TA + TB`),
//   * a non-dyn-compatible principal (`dyn NotObj`, a generic method),
//   * a `?`-modified bound (`dyn ?TA`),
//   * a non-type generic argument on the principal (`dyn Get<3>`).

use std::boxed::Box;
use std::marker::Send;

trait TA {
    fn a(&self) -> u8;
}
trait TB {
    fn b(&self) -> u8;
}
trait Get<T> {
    fn g(&self) -> T;
}
trait NotObj {
    fn method<U>(&self, u: U);
}

struct S<T>(T);
trait Probe {
    fn probe(&self) -> u32;
}

// A real, valid impl: resolution must still decline `dyn Send` (no principal)
// rather than match this candidate.
impl Probe for S<Box<dyn Send>> {
    #[kani::ensures(|r| *r == 9)]
    fn probe(&self) -> u32 {
        9
    }
}

#[kani::proof_for_contract(<S<Box<dyn Send>> as Probe>::probe)]
fn check_marker_only() {
    let s = S(Box::new(0u8) as Box<dyn Send>);
    let _ = s.probe();
}

#[kani::proof_for_contract(<S<Box<dyn TA + TB>> as Probe>::probe)]
fn check_two_principals() {}

#[kani::proof_for_contract(<S<Box<dyn NotObj>> as Probe>::probe)]
fn check_non_dyn_compatible() {}

#[kani::proof_for_contract(<S<Box<dyn ?TA>> as Probe>::probe)]
fn check_maybe_modified() {}

#[kani::proof_for_contract(<S<Box<dyn Get<3>>> as Probe>::probe)]
fn check_non_type_argument() {}
