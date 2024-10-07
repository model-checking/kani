// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

//! Tests for handling potentially uninitialized memory access via unions.
//! TODO: add a `.expected` file for this test.

use std::ptr::addr_of;

#[repr(C)]
#[derive(Clone, Copy)]
union U {
    a: u16,
    b: u32,
}

/// Reading non-padding data but a union is behind a pointer.
#[kani::proof]
unsafe fn pointer_union_should_pass() {
    let u = U { a: 0 }; // `u` is initialized for 2 bytes.
    let u_ptr = addr_of!(u);
    let u1 = *u_ptr;
    let padding = u1.a; // Read 2 bytes from `u`.
}

/// Reading padding data but a union is behind a pointer.
#[kani::proof]
unsafe fn pointer_union_should_fail() {
    let u = U { a: 0 }; // `u` is initialized for 2 bytes.
    let u_ptr = addr_of!(u);
    let u1 = *u_ptr;
    let padding = u1.b; // Read 4 bytes from `u`.
}

#[repr(C)]
struct S {
    u: U,
}

/// Tests uninitialized access if unions are top-level subfields.
#[kani::proof]
unsafe fn union_as_subfields_should_pass() {
    let u = U { a: 0 }; // `u` is initialized for 2 bytes.
    let s = S { u };
    let s1 = s;
    let u1 = s1.u; // `u1` is initialized for 2 bytes.
    let padding = u1.a; // Read 2 bytes from `u`.
}

/// Tests initialized access if unions are top-level subfields.
#[kani::proof]
unsafe fn union_as_subfields_should_fail() {
    let u = U { a: 0 }; // `u` is initialized for 2 bytes.
    let s = S { u };
    let s1 = s;
    let u1 = s1.u; // `u1` is initialized for 2 bytes.
    let padding = u1.b; // Read 4 bytes from `u`.
}

union Outer {
    u: U,
    a: u32,
}

/// Tests unions composing with other unions and reading non-padding data.
#[kani::proof]
unsafe fn uber_union_should_pass() {
    let u = Outer { u: U { b: 0 } }; // `u` is initialized for 4 bytes.
    let non_padding = u.a; // Read 4 bytes from `u`.
}

/// Tests unions composing with other unions and reading padding data.
#[kani::proof]
unsafe fn uber_union_should_fail() {
    let u = Outer { u: U { a: 0 } }; // `u` is initialized for 2 bytes.
    let padding = u.a; // Read 4 bytes from `u`.
}

/// Attempting to read initialized data via transmuting a union.
#[kani::proof]
unsafe fn transmute_union_should_pass() {
    let u = U { b: 0 }; // `u` is initialized for 4 bytes.
    let non_padding: u32 = std::mem::transmute(u); // Transmute `u` into a value of 4 bytes.
}

/// Attempting to read uninitialized data via transmuting a union.
#[kani::proof]
unsafe fn transmute_union_should_fail() {
    let u = U { a: 0 }; // `u` is initialized for 2 bytes.
    let padding: u32 = std::mem::transmute(u); // Transmute `u` into a value of 4 bytes.
}

/// Attempting to transmute into union and read initialized data.
#[kani::proof]
unsafe fn transmute_into_union_should_pass() {
    let u: U = std::mem::transmute(0u32); // `u` is initialized for 4 bytes.
    let non_padding = u.b; // Read 4 bytes from `u`.
}

/// Attempting to transmute into union and read uninitialized data.
#[kani::proof]
unsafe fn transmute_into_union_should_fail() {
    let u: U = std::mem::transmute_copy(&0u16); // `u` is initialized for 2 bytes.
    let padding = u.b; // Read 4 bytes from `u`.
}
