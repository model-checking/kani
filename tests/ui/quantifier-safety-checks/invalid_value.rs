// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers -Z valid-value-checks --solver cadical
#![feature(core_intrinsics)]

#[kani::proof]
fn invalid_predicate() {
    let bytes = [3u8; 2];
    let ptr = bytes.as_ptr().cast::<bool>();
    let enabled = true;
    assert!(kani::internal::kani_forall(0, 2, move |i: usize| {
        kani::internal::kani_exists(0, 2, move |j: usize| unsafe {
            *core::intrinsics::arith_offset(ptr, i as isize) & (i != j) & enabled
        })
    }));
}
