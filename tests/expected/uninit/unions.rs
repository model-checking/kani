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
    let non_padding = u1.a;
    assert!(non_padding == 0);
}

/// Reading padding data via simple union access.
#[kani::proof]
unsafe fn basic_union_should_fail() {
    let u = U { a: 0 };
    let u1 = u;
    let padding = u1.b;
}

#[repr(C)]
#[derive(Clone, Copy)]
union MultiU {
    d: u128,
    a: u16,
    c: u64,
    b: u32,
}

/// Simple and correct multifield union access.
#[kani::proof]
unsafe fn basic_multifield_union_should_pass() {
    let u = MultiU { c: 0 };
    let mut u1 = u;
    let non_padding_a = u1.a;
    assert!(non_padding_a == 0);
    let non_padding_b = u1.b;
    assert!(non_padding_b == 0);
    let non_padding_c = u1.c;
    assert!(non_padding_c == 0);
}

/// Reading padding data via simple multifield union access.
#[kani::proof]
unsafe fn basic_multifield_union_should_fail() {
    let u = MultiU { c: 0 };
    let mut u1 = u;
    let non_padding_a = u1.a;
    assert!(non_padding_a == 0);
    let non_padding_b = u1.b;
    assert!(non_padding_b == 0);
    let non_padding_c = u1.c;
    assert!(non_padding_c == 0);
    let padding = u1.d; // Accessing uninitialized data.
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
    let padding = u.b.1;
}

/// Tests overwriting data inside unions.
#[kani::proof]
unsafe fn union_update_should_pass() {
    let mut u = U { a: 0 };
    u.b = 0;
    let non_padding = u.b;
    assert!(non_padding == 0);
}

/// Tests overwriting data inside unions.
#[kani::proof]
unsafe fn union_update_should_fail() {
    let mut u = U { a: 0 };
    u.a = 0;
    let padding = u.b;
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

/// Reading non-padding data via simple union access if union is passed to another function.
#[kani::proof]
unsafe fn cross_function_union_should_pass() {
    unsafe fn helper(u: U) {
        let non_padding = u.a; // Read 2 bytes from `u`.
    }
    let u = U { a: 0 }; // `u` is initialized for 2 bytes.
    helper(u);
}

/// Reading padding data via simple union access if union is passed to another function multiple
/// times.
#[kani::proof]
unsafe fn multi_cross_function_union_should_fail() {
    unsafe fn helper(u: U) {
        sub_helper(u);
    }
    unsafe fn sub_helper(u: U) {
        let padding = u.b; // Read 4 bytes from `u`.
    }
    let u = U { a: 0 }; // `u` is initialized for 2 bytes.
    helper(u);
}

/// Reading non-padding data via simple union access if union is passed to another function multiple
/// times.
#[kani::proof]
unsafe fn multi_cross_function_union_should_pass() {
    unsafe fn helper(u: U) {
        sub_helper(u);
    }
    unsafe fn sub_helper(u: U) {
        let non_padding = u.a; // Read 2 bytes from `u`.
    }
    let u = U { a: 0 }; // `u` is initialized for 2 bytes.
    helper(u);
}

/// Reading padding data via simple union access if multiple unions are passed to another function.
#[kani::proof]
unsafe fn cross_function_multi_union_should_fail() {
    unsafe fn helper(u1: U, u2: U) {
        let padding = u1.b; // Read 4 bytes from `u1`.
        let non_padding = u2.b; // Read 4 bytes from `u2`.
    }
    let u1 = U { a: 0 }; // `u1` is initialized for 2 bytes.
    let u2 = U { b: 0 }; // `u2` is initialized for 4 bytes.
    helper(u1, u2);
}

/// Reading non-padding data via simple union access if multiple unions are passed to another
/// function.
#[kani::proof]
unsafe fn cross_function_multi_union_should_pass() {
    unsafe fn helper(u1: U, u2: U) {
        let padding = u1.b; // Read 4 bytes from `u1`.
        let non_padding = u2.b; // Read 4 bytes from `u2`.
    }
    let u1 = U { b: 0 }; // `u1` is initialized for 4 bytes.
    let u2 = U { b: 0 }; // `u2` is initialized for 4 bytes.
    helper(u1, u2);
}
