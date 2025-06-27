// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that Kani can automatically derive `Invariant` for empty structs.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
struct Void;

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
struct Void2(());

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
struct VoidOfVoid(Void, Void2);

#[kani::proof]
fn check_empty_struct_invariant_1() {
    let void1: Void = kani::any();
    assert!(void1.is_safe());
}

#[kani::proof]
fn check_empty_struct_invariant_2() {
    let void2: Void2 = kani::any();
    assert!(void2.is_safe());
}

#[kani::proof]
fn check_empty_struct_invariant_3() {
    let void3: VoidOfVoid = kani::any();
    assert!(void3.is_safe());
}
