// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// compile-flags: --edition 2021
// An over-aligned ZST still requires alignment even though no memory is read.
#![feature(core_intrinsics)]

#[repr(align(8))]
struct Z;

#[kani::proof]
fn misaligned_zst() {
    let src = std::ptr::without_provenance::<Z>(1);
    let _ = unsafe { std::intrinsics::volatile_load(src) };
}
