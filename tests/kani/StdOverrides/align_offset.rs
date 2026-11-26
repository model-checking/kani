// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test makes sure Kani uses its hook for align_offset.

#[kani::proof]
fn align_offset() {
    let x = [10u8; 8];
    let base_ptr = x.as_ptr();
    // Offset 0 within the object is a multiple of any alignment, so this is already aligned.
    assert_eq!(base_ptr.align_offset(4), 0);

    // Offset 1 is not a multiple of 4. The hook answers `usize::MAX`, which `align_offset` is
    // explicitly permitted to return; the real implementation would answer 3. Asserting
    // `usize::MAX` here is what shows the hook is being used rather than the real implementation.
    let unaligned_ptr = unsafe { base_ptr.add(1) };
    assert_eq!(unaligned_ptr.align_offset(4), usize::MAX);
}
