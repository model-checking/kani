// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z stubbing

//! Test that applying multiple `#[kani::use_stub_set(...)]` attributes to
//! the same harness works — each attribute's stubs are all applied.

fn real_a() -> u32 {
    0
}
fn mock_a() -> u32 {
    10
}

fn real_b() -> u32 {
    0
}
fn mock_b() -> u32 {
    20
}

fn real_c() -> u32 {
    0
}
fn mock_c() -> u32 {
    30
}

kani::stub_set!(set_a, stub(real_a, mock_a),);
kani::stub_set!(set_b, stub(real_b, mock_b),);
kani::stub_set!(set_c, stub(real_c, mock_c),);

#[kani::proof]
#[kani::use_stub_set(set_a)]
#[kani::use_stub_set(set_b)]
#[kani::use_stub_set(set_c)]
fn check_multiple_use_stub_set() {
    assert_eq!(real_a(), 10);
    assert_eq!(real_b(), 20);
    assert_eq!(real_c(), 30);
}

/// Combining multiple `use_stub_set` attributes with an individual `#[kani::stub]`.
#[kani::proof]
#[kani::use_stub_set(set_a)]
#[kani::use_stub_set(set_b)]
#[kani::stub(real_c, mock_c)]
fn check_multiple_use_stub_set_with_individual() {
    assert_eq!(real_a(), 10);
    assert_eq!(real_b(), 20);
    assert_eq!(real_c(), 30);
}
