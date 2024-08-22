// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

//! Tests for handling potentially uninitialized memory access via unions.

use std::ptr::addr_of;

#[repr(C)]
#[derive(Clone, Copy)]
union U {
    a: u16,
    b: u32,
}

/// Simple and correct union access.
#[kani::proof]
unsafe fn basic_union_should_pass() {
    let u = U { b: 0 };
    let u1 = u;
    let padding = u1.a;
}

/// Reading padding data via simple union access.
#[kani::proof]
unsafe fn basic_union_should_fail() {
    let u = U { a: 0 };
    let u1 = u;
    let padding = u1.b;
}

#[repr(C)]
union U1 {
    a: (u32, u8),
    b: (u32, u16, u8),
}

/// Tests accessing initialized data via subfields of a union.
#[kani::proof]
unsafe fn union_complex_subfields_should_pass() {
    let u = U1 { a: (0, 0) };
    let non_padding = u.b.0;
}

/// Tests accessing uninit data via subfields of a union.
#[kani::proof]
unsafe fn union_complex_subfields_should_fail() {
    let u = U1 { a: (0, 0) };
    let non_padding = u.b.1;
}

/// Tests overwriting data inside unions.
#[kani::proof]
unsafe fn union_update_should_pass() {
    let mut u = U { a: 0 };
    u.b = 0;
    let non_padding = u.b;
}

/// Tests overwriting data inside unions.
#[kani::proof]
unsafe fn union_update_should_fail() {
    let mut u = U { a: 0 };
    u.a = 0;
    let non_padding = u.b;
}
