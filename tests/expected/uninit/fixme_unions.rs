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

/// Reading padding data via simple union access if union is passed to another function.
#[kani::proof]
unsafe fn cross_function_union_should_fail() {
    unsafe fn helper(u: U) {
        let padding = u.b; // Read 4 bytes from `u`.
    }
    let u = U { a: 0 }; // `u` is initialized for 2 bytes.
    helper(u);
}

/// Reading padding data but a union is via a union from behind a pointer.
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

union Outer {
    u: U,
    a: u32,
}

/// Tests unions composing with other unions.
#[kani::proof]
unsafe fn uber_union_should_pass() {
    let u = Outer { u: U { b: 0 } }; // `u` is initialized for 4 bytes.
    let non_padding = u.a; // Read 4 bytes from `u`.
}
