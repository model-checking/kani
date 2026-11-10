// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers -Z valid-value-checks --solver cadical
#![feature(core_intrinsics)]
use std::mem::MaybeUninit;

#[kani::proof]
fn invalid_capture() {
    let bits = unsafe { MaybeUninit::new(3u8).assume_init() };
    let enabled: bool = unsafe { std::mem::transmute(bits) };
    assert!(kani::internal::kani_forall(0, 2, move |i: usize| {
        kani::internal::kani_exists(0, 2, move |j: usize| (i != j) & enabled)
    }));
}

#[kani::proof]
fn valid_capture() {
    let enabled = unsafe { MaybeUninit::new(true).assume_init() };
    assert!(kani::internal::kani_forall(0, 2, move |i: usize| {
        kani::internal::kani_exists(0, 2, move |j: usize| (i != j) & enabled)
    }));
}
