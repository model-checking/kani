// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers -Z uninit-checks --solver cadical
#![feature(core_intrinsics)]
#[inline(never)]
unsafe fn read_byte(ptr: *const u8) -> u8 {
    unsafe { *ptr }
}
#[kani::proof]
fn uninitialized_helper() {
    let data = Vec::<u8>::with_capacity(2);
    let ptr = data.as_ptr();
    assert!(kani::internal::kani_forall(0, 2, move |i: usize| unsafe {
        let _ = read_byte(core::intrinsics::arith_offset(ptr, i as isize));
        true
    }));
}
