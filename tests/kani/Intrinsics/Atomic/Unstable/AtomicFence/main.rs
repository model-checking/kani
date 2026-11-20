// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `atomic_fence` and other variants (unstable version) can be
// processed.

#![feature(core_intrinsics)]
use std::intrinsics::{AtomicOrdering, atomic_fence};

#[kani::proof]
fn main() {
    unsafe {
        atomic_fence::<{ AtomicOrdering::SeqCst }>();
        atomic_fence::<{ AtomicOrdering::Acquire }>();
        atomic_fence::<{ AtomicOrdering::AcqRel }>();
        atomic_fence::<{ AtomicOrdering::Release }>();
    }
}
