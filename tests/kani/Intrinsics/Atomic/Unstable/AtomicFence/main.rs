// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `atomic_fence` and other variants (unstable version) can be
// processed.

#![feature(core_intrinsics)]
use std::intrinsics::{
    atomic_fence_acqrel, atomic_fence_acquire, atomic_fence_release, atomic_fence_seqcst,
};

#[kani::proof]
fn main() {
    unsafe {
        atomic_fence_seqcst();
        atomic_fence_acquire();
        atomic_fence_acqrel();
        atomic_fence_release();
    }
}
