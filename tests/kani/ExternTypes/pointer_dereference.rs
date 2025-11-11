// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z c-ffi
//! This test checks that Kani can handle code generation for extern types
//! without hitting an internal compiler panic (index out of bounds).
//! The size_and_align_of_dst function should handle extern types gracefully.
//! This is mainly a compilation test - the code doesn't actually execute problematic operations.

#![feature(extern_types)]

extern "C" {
    type ExternType;
}

#[kani::proof]
fn check_extern_type_compiles() {
    // Just verify we can work with pointers to extern types
    let _ptr: *const ExternType = 0x1000 as *const ExternType;
    // We don't dereference it, just testing that the compiler can handle the type
}
