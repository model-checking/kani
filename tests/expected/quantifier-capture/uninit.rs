// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers -Z uninit-checks --solver cadical
#![feature(core_intrinsics)]

#[kani::proof]
fn uninitialized_capture() {
    let source = Vec::<u8>::with_capacity(2);
    let value = unsafe { *source.as_ptr() };
    assert!(kani::internal::kani_forall(0, 2, move |i: usize| {
        kani::internal::kani_exists(0, 2, move |j: usize| (i != j) & (value < 255))
    }));
}

#[kani::proof]
fn initialized_capture() {
    let source = vec![1u8; 2];
    let value = unsafe { *source.as_ptr() };
    assert!(kani::internal::kani_forall(0, 2, move |i: usize| {
        kani::internal::kani_exists(0, 2, move |j: usize| (i != j) & (value == 1))
    }));
}
