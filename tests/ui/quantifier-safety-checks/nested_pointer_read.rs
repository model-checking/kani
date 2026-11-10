// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers --solver cadical
#![feature(core_intrinsics)]

#[kani::proof]
fn nested_pointer_read() {
    let values = [0u8, 1];
    let ptr = values.as_ptr();
    assert!(kani::forall!(|i in (0, 2)| {
        kani::exists!(|j in (0, 2)| unsafe {
            *core::intrinsics::arith_offset(ptr, i as isize)
                != *core::intrinsics::arith_offset(ptr, j as isize)
        })
    }));
}
