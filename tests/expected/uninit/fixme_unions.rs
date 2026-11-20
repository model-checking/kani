// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

//! Union cases `-Z uninit-checks` does not handle yet: unions nested in unions, and
//! transmutes into a union (https://github.com/model-checking/kani/issues/5151).
//! TODO: add a `.expected` file for this test.

#[repr(C)]
#[derive(Clone, Copy)]
union U {
    a: u16,
    b: u32,
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
