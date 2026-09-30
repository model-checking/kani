// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Regression test: ptr.wrapping_add(usize::MAX) should not be flagged as UB
// under default settings. wrapping_add is explicitly defined in Rust as
// performing wrapping pointer arithmetic, which is always safe (though the
// resulting pointer may not be dereferenceable).
//
// Note: With --extra-pointer-checks (unstable), CBMC's --pointer-overflow-check
// may produce a false positive for this case. This is a known limitation of
// the unstable feature.

/// Verify that wrapping_add with extreme offsets is not UB.
#[kani::proof]
fn pointer_wrapping_add_max() {
    let data = [0u8; 1];
    let ptr = data.as_ptr();
    // wrapping_add is always safe, even with extreme values
    let wrapped = ptr.wrapping_add(usize::MAX);
    // The wrapped pointer should differ from the original
    // (unless the address space wraps perfectly, which is platform-dependent)
    let _ = wrapped;
}

/// Verify that wrapping_sub with extreme offsets is not UB.
#[kani::proof]
fn pointer_wrapping_sub_max() {
    let data = [0u8; 1];
    let ptr = data.as_ptr();
    let wrapped = ptr.wrapping_sub(usize::MAX);
    let _ = wrapped;
}

/// Verify wrapping_add with a moderate offset beyond allocation bounds.
#[kani::proof]
fn pointer_wrapping_add_beyond_alloc() {
    let data = [0u8; 4];
    let ptr = data.as_ptr();
    // Offset beyond allocation but within address space — still not UB for wrapping_add
    let wrapped = ptr.wrapping_add(1000);
    let _ = wrapped;
}
