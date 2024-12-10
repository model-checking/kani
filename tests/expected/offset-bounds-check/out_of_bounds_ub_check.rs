// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani offset operations correctly detect out-of-bound access.

/// Verification should fail because safety violation is not a regular panic.
#[kani::proof]
#[kani::should_panic]
fn check_ptr_oob() {
    let array = [0];
    let base_ptr = array.as_ptr();
    // SAFETY: This is unsafe and it will trigger UB.
    let oob_ptr = unsafe { base_ptr.sub(1) };
    // Just use the pointer to avoid warnings
    assert_ne!(oob_ptr.addr(), base_ptr.addr());
}

/// Verification should succeed.
#[kani::proof]
fn check_ptr_end() {
    let array = [0];
    let base_ptr = array.as_ptr();
    // Safety: This should be OK since the pointer is pointing to the end of the allocation.
    let end_ptr = unsafe { base_ptr.add(1) };
    // Safety: Pointers point to the same allocation
    assert_eq!(unsafe { end_ptr.offset_from(base_ptr) }, 1);
}
