// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `atomic_singlethreadfence_seqcst` and other variants (unstable version)
// can be processed.

#![feature(core_intrinsics)]
use std::intrinsics::{
    atomic_singlethreadfence_acqrel, atomic_singlethreadfence_acquire,
    atomic_singlethreadfence_release, atomic_singlethreadfence_seqcst,
};

#[kani::proof]
fn main() {
    unsafe {
        atomic_singlethreadfence_seqcst();
        atomic_singlethreadfence_acquire();
        atomic_singlethreadfence_acqrel();
        atomic_singlethreadfence_release();
    }
}
