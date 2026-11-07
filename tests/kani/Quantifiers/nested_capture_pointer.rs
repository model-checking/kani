// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers --solver cadical
#![feature(core_intrinsics)]

#[kani::proof]
fn padded_capture() {
    let bytes: [u8; 2] = kani::any();
    kani::assume(bytes[0] != bytes[1]);
    let enabled: bool = kani::any();
    kani::assume(enabled);
    let ptr = bytes.as_ptr();
    assert!(kani::internal::kani_forall(0, 2, move |i: usize| {
        kani::internal::kani_exists(0, 2, move |j: usize| unsafe {
            (*core::intrinsics::arith_offset(ptr, i as isize)
                != *core::intrinsics::arith_offset(ptr, j as isize))
                & enabled
        })
    }));
}

#[kani::proof]
#[kani::should_panic]
fn no_single_pointer_witness() {
    let bytes: [u8; 2] = kani::any();
    kani::assume(bytes[0] != bytes[1]);
    let enabled: bool = kani::any();
    kani::assume(enabled);
    let ptr = bytes.as_ptr();
    assert!(kani::internal::kani_exists(0, 2, move |j: usize| {
        kani::internal::kani_forall(0, 2, move |i: usize| unsafe {
            (*core::intrinsics::arith_offset(ptr, i as isize)
                == *core::intrinsics::arith_offset(ptr, j as isize))
                & enabled
        })
    }));
}

#[kani::proof]
fn public_pointer_capture() {
    let bytes: [u8; 2] = kani::any();
    kani::assume(bytes[0] != bytes[1]);
    let enabled: bool = kani::any();
    kani::assume(enabled);
    let ptr = bytes.as_ptr();
    assert!(kani::forall!(|i in (0, 2)| {
        kani::exists!(|j in (0, 2)| unsafe {
            (*core::intrinsics::arith_offset(ptr, i as isize)
                != *core::intrinsics::arith_offset(ptr, j as isize))
                & enabled
        })
    }));
}

#[kani::proof]
#[kani::should_panic]
fn public_no_single_pointer_witness() {
    let bytes: [u8; 2] = kani::any();
    kani::assume(bytes[0] != bytes[1]);
    let enabled: bool = kani::any();
    kani::assume(enabled);
    let ptr = bytes.as_ptr();
    assert!(kani::exists!(|j in (0, 2)| {
        kani::forall!(|i in (0, 2)| unsafe {
            (*core::intrinsics::arith_offset(ptr, i as isize)
                == *core::intrinsics::arith_offset(ptr, j as isize))
                & enabled
        })
    }));
}
