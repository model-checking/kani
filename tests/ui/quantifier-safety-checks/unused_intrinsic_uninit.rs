// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers -Z uninit-checks --solver cadical

#![feature(core_intrinsics)]
#[kani::proof]
fn sqrt_unused() {
    assert!(kani::internal::kani_forall(0, 2, |i: usize| {
        let _ = unsafe { core::intrinsics::sqrtf64(i as f64) };
        i < 2
    }));
}
