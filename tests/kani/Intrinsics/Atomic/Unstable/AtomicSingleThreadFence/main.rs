// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `atomic_singlethreadfence_seqcst` and other variants (unstable version)
// can be processed.

#![feature(core_intrinsics)]
use std::intrinsics::{AtomicOrdering, atomic_singlethreadfence};

#[kani::proof]
fn main() {
    unsafe {
        atomic_singlethreadfence::<{ AtomicOrdering::SeqCst }>();
        atomic_singlethreadfence::<{ AtomicOrdering::Acquire }>();
        atomic_singlethreadfence::<{ AtomicOrdering::AcqRel }>();
        atomic_singlethreadfence::<{ AtomicOrdering::Release }>();
    }
}
