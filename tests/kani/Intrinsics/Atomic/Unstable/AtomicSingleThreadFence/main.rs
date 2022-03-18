// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `atomic_singlethreadfence` and other variants (unstable version)
// can be processed.

#![feature(core_intrinsics)]
use std::intrinsics::{
    atomic_singlethreadfence, atomic_singlethreadfence_acq, atomic_singlethreadfence_acqrel,
    atomic_singlethreadfence_rel,
};

#[kani::proof]
fn main() {
    unsafe {
        atomic_singlethreadfence();
        atomic_singlethreadfence_acq();
        atomic_singlethreadfence_acqrel();
        atomic_singlethreadfence_rel();
    }
}
