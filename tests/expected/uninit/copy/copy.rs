// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

#[repr(C)]
#[derive(kani::Arbitrary)]
struct S(u32, u8); // 5 bytes of data + 3 bytes of padding.

#[kani::proof]
/// This checks that reading copied uninitialized bytes fails an assertion.
unsafe fn expose_padding_via_copy() {
    let from: S = kani::any();
    let mut to: u64 = kani::any();

    let from_ptr = &from as *const S;
    let to_ptr = &mut to as *mut u64;

    // This should not cause UB since `copy` is untyped.
    std::ptr::copy(from_ptr as *const u8, to_ptr as *mut u8, std::mem::size_of::<S>()); 

    // This reads uninitialized bytes, which is UB.
    let padding: u64 = std::ptr::read(to_ptr);
}

#[kani::proof]
/// This checks that reading copied uninitialized bytes after a multi-byte copy fails an assertion.
unsafe fn expose_padding_via_non_byte_copy() {
    let from: S = kani::any();
    let mut to: u64 = kani::any();

    let from_ptr = &from as *const S;
    let to_ptr = &mut to as *mut u64;

    // This should not cause UB since `copy` is untyped.
    std::ptr::copy(from_ptr as *const u64, to_ptr as *mut u64, 1); 

    // This reads uninitialized bytes, which is UB.
    let padding: u64 = std::ptr::read(to_ptr);
}

#[kani::proof]
/// This checks that reading copied initialized bytes verifies correctly.
unsafe fn copy_without_padding() {
    let from: S = kani::any();
    let mut to: u64 = kani::any();

    let from_ptr = &from as *const S;
    let to_ptr = &mut to as *mut u64;

    // This should not cause UB since `copy` is untyped.
    std::ptr::copy(from_ptr as *const u8, to_ptr as *mut u8, std::mem::size_of::<u32>());

    // Since the previous copy only copied 4 bytes, no padding was copied, so no padding is read.
    let data: u64 = std::ptr::read(to_ptr); 
}

#[kani::proof]
/// This checks that reading copied initialized bytes after a multi-byte copy verifies correctly.
unsafe fn non_byte_copy_without_padding() {
    let from: S = kani::any();
    let mut to: u64 = kani::any();

    let from_ptr = &from as *const S;
    let to_ptr = &mut to as *mut u64;

    // This should not cause UB since `copy` is untyped.
    std::ptr::copy(from_ptr as *const u32, to_ptr as *mut u32, 1);

    // Since the previous copy only copied 4 bytes, no padding was copied, so no padding is read.
    let data: u64 = std::ptr::read(to_ptr); 
}

#[kani::proof]
/// This checks that reading uninitialized bytes fails an assertion even after copy.
unsafe fn read_after_copy() {
    let from: S = kani::any();
    let mut to: u64 = kani::any();

    let from_ptr = &from as *const S;
    let to_ptr = &mut to as *mut u64;

    // This should not cause UB since `copy` is untyped.
    std::ptr::copy(from_ptr as *const u8, to_ptr as *mut u8, std::mem::size_of::<u64>());

    // Reading padding from the previous place should be UB even after copy.
    let data: u64 = std::ptr::read(from_ptr as *const u64); 
}
