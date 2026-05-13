// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z stubbing

//! Test that diamond-shaped stub set composition works correctly.
//! Both set_a and set_b include shared_set — this should NOT produce
//! a circular reference error.

fn real_shared() -> u32 {
    0
}
fn mock_shared() -> u32 {
    1
}

fn real_a() -> u32 {
    0
}
fn mock_a() -> u32 {
    2
}

fn real_b() -> u32 {
    0
}
fn mock_b() -> u32 {
    3
}

kani::stub_set!(shared_set, stub(real_shared, mock_shared),);
kani::stub_set!(set_a, stub(real_a, mock_a), use_stub_set(shared_set),);
kani::stub_set!(set_b, stub(real_b, mock_b), use_stub_set(shared_set),);
kani::stub_set!(combined, use_stub_set(set_a), use_stub_set(set_b),);

#[kani::proof]
#[kani::use_stub_set(combined)]
fn check_diamond_composition() {
    assert_eq!(real_shared(), 1);
    assert_eq!(real_a(), 2);
    assert_eq!(real_b(), 3);
}
