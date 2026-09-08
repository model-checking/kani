// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Nondeterministic `Rc<T>`/`Arc<T>` values must cover every behavioral equivalence class of
//! reference-count state: unique vs. shared strong ownership, and presence vs. absence of weak
//! references (https://github.com/model-checking/kani/issues/4752). Count observers only branch
//! on uniqueness, so `strong_count == 2` represents every shared state and `weak_count == 1`
//! every state with weak references.

use std::rc::Rc;
use std::sync::Arc;

// --- Validity: the generated reference is always a valid, countable handle. ---

#[kani::proof]
fn rc_strong_count_at_least_one() {
    let rc: Rc<u8> = kani::any();
    assert!(Rc::strong_count(&rc) >= 1);
}

#[kani::proof]
fn arc_strong_count_at_least_one() {
    let arc: Arc<u8> = kani::any();
    assert!(Arc::strong_count(&arc) >= 1);
}

// --- Reachability: shared ownership and weak references are both generated.
// Each `should_panic` harness asserts the old (incomplete) behavior; it must FAIL, proving
// the newly covered class is reachable.

#[kani::proof]
#[kani::should_panic]
fn rc_shared_state_reachable() {
    let rc: Rc<u8> = kani::any();
    assert_eq!(Rc::strong_count(&rc), 1);
}

#[kani::proof]
#[kani::should_panic]
fn rc_weak_state_reachable() {
    let rc: Rc<u8> = kani::any();
    assert_eq!(Rc::weak_count(&rc), 0);
}

#[kani::proof]
#[kani::should_panic]
fn arc_shared_state_reachable() {
    let arc: Arc<u8> = kani::any();
    assert_eq!(Arc::strong_count(&arc), 1);
}

#[kani::proof]
#[kani::should_panic]
fn arc_weak_state_reachable() {
    let arc: Arc<u8> = kani::any();
    assert_eq!(Arc::weak_count(&arc), 0);
}

// --- Count-dependent APIs observe both outcomes. ---

#[kani::proof]
#[kani::should_panic]
fn rc_get_mut_can_fail() {
    let mut rc: Rc<u8> = kani::any();
    assert!(Rc::get_mut(&mut rc).is_some());
}

#[kani::proof]
#[kani::should_panic]
fn arc_get_mut_can_fail() {
    let mut arc: Arc<u8> = kani::any();
    assert!(Arc::get_mut(&mut arc).is_some());
}

// Uniqueness still implies exclusive access: the one case where `get_mut` must succeed.
#[kani::proof]
fn rc_get_mut_succeeds_when_unique() {
    let mut rc: Rc<u8> = kani::any();
    kani::assume(Rc::strong_count(&rc) == 1 && Rc::weak_count(&rc) == 0);
    assert!(Rc::get_mut(&mut rc).is_some());
}

#[kani::proof]
fn arc_get_mut_succeeds_when_unique() {
    let mut arc: Arc<u8> = kani::any();
    kani::assume(Arc::strong_count(&arc) == 1 && Arc::weak_count(&arc) == 0);
    assert!(Arc::get_mut(&mut arc).is_some());
}

// --- The pointee remains fully nondeterministic under sharing. ---

#[kani::proof]
#[kani::should_panic]
fn rc_pointee_extreme_values_generated() {
    let rc: Rc<u8> = kani::any();
    kani::cover!(*rc == 255);
    assert!(*rc < 255);
}

#[kani::proof]
#[kani::should_panic]
fn arc_pointee_extreme_values_generated() {
    let arc: Arc<u8> = kani::any();
    kani::cover!(*arc == 255);
    assert!(*arc < 255);
}
