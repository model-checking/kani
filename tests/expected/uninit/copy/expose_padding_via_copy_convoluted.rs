// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

#[repr(C)]
#[derive(kani::Arbitrary)]
struct S(u32, u8); // 5 bytes of data + 3 bytes of padding.


/// This checks that reading copied uninitialized bytes fails an assertion even if pointer are
/// passed around different functions.
#[kani::proof]
unsafe fn expose_padding_via_copy_convoluted() {
    unsafe fn copy_and_read_helper(from_ptr: *const S, to_ptr: *mut u64) -> u64 {
        // This should not cause UB since `copy` is untyped.
        std::ptr::copy(from_ptr as *const u8, to_ptr as *mut u8, std::mem::size_of::<S>());
        // This reads uninitialized bytes, which is UB.
        let padding: u64 = std::ptr::read(to_ptr);
        padding
    }

    unsafe fn partial_copy_and_read_helper(from_ptr: *const S, to_ptr: *mut u64) -> u32 {
        // This should not cause UB since `copy` is untyped.
        std::ptr::copy(from_ptr as *const u8, to_ptr as *mut u8, std::mem::size_of::<u32>());
        // This does not read uninitialized bytes.
        let not_padding: u32 = std::ptr::read(to_ptr as *mut u32);
        not_padding
    }

    let flag: bool = kani::any();

    let from: S = kani::any();
    let mut to: u64 = kani::any();

    let from_ptr = &from as *const S;
    let to_ptr = &mut to as *mut u64;

    if flag {
        copy_and_read_helper(from_ptr, to_ptr);
    } else {
        partial_copy_and_read_helper(from_ptr, to_ptr);
    }
}
