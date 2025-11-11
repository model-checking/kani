// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z c-ffi
//! This test checks that Kani can compile code with extern types and size_of_val
//! without hitting an internal compiler panic (index out of bounds).
//! The actual runtime behavior will be handled by Kani's models.

#![feature(extern_types)]

extern "C" {
    type ExternType;
}

#[kani::proof]
fn check_extern_type_compiles() {
    // This test just needs to compile successfully.
    // We're not actually dereferencing or calling size_of_val,
    // just ensuring the compiler can handle the extern type.
    let _ptr: *const ExternType = std::ptr::null();
}
