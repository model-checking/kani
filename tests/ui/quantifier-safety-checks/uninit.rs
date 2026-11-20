// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers -Z uninit-checks --solver cadical
#![feature(core_intrinsics)]

#[kani::proof]
fn uninitialized_predicate() {
    let source = Vec::<u8>::with_capacity(2);
    let ptr = source.as_ptr();
    let enabled = true;
    assert!(kani::internal::kani_forall(0, 2, move |i: usize| {
        kani::internal::kani_exists(0, 2, move |j: usize| unsafe {
            (*core::intrinsics::arith_offset(ptr, i as isize)
                == *core::intrinsics::arith_offset(ptr, j as isize))
                & enabled
        })
    }));
}
