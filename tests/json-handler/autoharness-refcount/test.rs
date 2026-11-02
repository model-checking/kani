// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Functions for `kani autoharness`, checked by test.sh: harnesses whose generated values
//! contain an `Rc`/`Arc` (directly or nested) are marked "(refcount)" / `is_refcount_bounded`,
//! since Kani models reference counts with finitely many states
//! (https://github.com/model-checking/kani/issues/4752); all others are not.
//! Every function verifies successfully, and every cover must be SATISFIED (test.sh checks
//! this, since an unsatisfiable cover alone does not fail verification).

use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

/// No `Arbitrary` implementation: `Rc<OnlyDerivable>` goes through the compiler-derived
/// `AnyRc` model rather than the blanket `Arbitrary for Rc<T>` impl.
pub struct OnlyDerivable {
    pub x: u8,
}

#[derive(kani::Arbitrary)]
pub struct HasArc {
    pub a: Arc<u8>,
}

/// Holds no `Rc` value: `PhantomData` is skipped.
#[derive(kani::Arbitrary)]
pub struct PhantomRc {
    pub x: u8,
    pub m: PhantomData<Rc<u8>>,
}

// --- Marked "(refcount)" ---

/// Blanket `Arbitrary for Rc<T>` path; the uniquely-owned state must be reachable.
pub fn rc_arg(r: Rc<u8>) -> u8 {
    kani::cover!(Rc::strong_count(&r) == 1 && Rc::weak_count(&r) == 0, "Rc: unique state");
    *r
}

/// AnyRc model path. The uniquely-owned state and the full pointee range must be reachable
/// (non-vacuity of the generated value, whatever the reference-count model).
pub fn rc_derivable(r: Rc<OnlyDerivable>) -> u8 {
    kani::cover!(Rc::strong_count(&r) == 1 && Rc::weak_count(&r) == 0, "AnyRc: unique state");
    kani::cover!(r.x == 255, "AnyRc: extreme pointee");
    r.x
}

/// AnyArc model path, as `rc_derivable`.
pub fn arc_derivable(a: Arc<OnlyDerivable>) -> u8 {
    kani::cover!(Arc::strong_count(&a) == 1 && Arc::weak_count(&a) == 0, "AnyArc: unique state");
    kani::cover!(a.x == 255, "AnyArc: extreme pointee");
    a.x
}

pub fn arc_field(s: HasArc) -> u8 {
    *s.a
}

pub fn rc_in_tuple(t: (u8, Rc<u8>)) -> u8 {
    t.0 ^ *t.1
}

pub fn arc_by_ref(a: &Arc<u8>) -> u8 {
    **a
}

pub fn rc_in_box(b: Box<Rc<u8>>) -> u8 {
    **b
}

// --- Not marked ---

pub fn plain(x: u8) -> u8 {
    x
}

pub fn boxed(b: Box<u8>) -> u8 {
    *b
}

pub fn phantom(p: PhantomRc) -> u8 {
    p.x
}
