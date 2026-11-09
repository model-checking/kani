// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers -Z valid-value-checks --solver cadical
#![feature(core_intrinsics)]
#[inline(never)]
unsafe fn read_bool(ptr: *const bool) -> bool {
    unsafe { *ptr }
}
#[kani::proof]
fn invalid_helper() {
    let data = [3u8; 2];
    let ptr = data.as_ptr().cast::<bool>();
    assert!(kani::internal::kani_forall(0, 2, move |i: usize| unsafe {
        read_bool(core::intrinsics::arith_offset(ptr, i as isize))
    }));
}
