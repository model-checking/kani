// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]
use std::intrinsics::{atomic_fence, atomic_fence_acq, atomic_fence_acqrel, atomic_fence_rel};

fn main() {
    unsafe {
        atomic_fence();
        atomic_fence_acq();
        atomic_fence_acqrel();
        atomic_fence_rel();
    }
}
