// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the autoharness subcommand supports Box<T>, Rc<T>, and Arc<T> arguments, both for
// pointee types that implement Arbitrary and for pointees whose Arbitrary implementation the
// compiler derives (via the AnyBox/AnyRc/AnyArc models). These values are *unbounded*: a smart
// pointer to T covers exactly the values of T, so no --bounded-arguments is needed.
// The "TEST NOTE" comments explain the expected result per function.

use std::rc::Rc;
use std::sync::Arc;

#[derive(kani::Arbitrary)]
pub struct Derived {
    pub x: u8,
}

// No Arbitrary implementation; the compiler derives one.
pub struct OnlyDerivable {
    pub x: u8,
}

// TEST NOTE: should PASS.
pub fn box_derived(b: Box<Derived>) -> u8 {
    b.x
}

// TEST NOTE: should PASS: the pointee's Arbitrary implementation is compiler-derived.
pub fn box_derivable(b: Box<OnlyDerivable>) -> u8 {
    b.x
}

// TEST NOTE: should PASS.
pub fn rc_derivable(r: Rc<OnlyDerivable>) -> u8 {
    r.x
}

// TEST NOTE: should PASS.
pub fn arc_derivable(a: Arc<OnlyDerivable>) -> u8 {
    a.x
}

// TEST NOTE: should FAIL, and the cover check must be SATISFIED: all pointee values are
// generated (full coverage; smart pointers are not bounded).
pub fn arc_assert(a: Arc<Derived>) {
    kani::cover!(a.x == 255, "extreme pointee values are generated");
    assert!(a.x < 255);
}

// TEST NOTE: skipped: pointees that can neither implement nor derive Arbitrary remain
// unsupported.
pub struct NotDerivable {
    pub p: *const u8,
}

pub fn box_not_derivable(b: Box<NotDerivable>) -> usize {
    b.p as usize
}
