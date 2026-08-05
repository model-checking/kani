// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! Test that different harnesses can use different stub sets.

fn real_fn() -> u32 {
    0
}

fn stub_one() -> u32 {
    1
}

fn stub_two() -> u32 {
    2
}

kani::stub_set!(set_one, stub(real_fn, stub_one),);

kani::stub_set!(set_two, stub(real_fn, stub_two),);

#[kani::proof]
#[kani::use_stub_set(set_one)]
fn check_harness_uses_set_one() {
    assert_eq!(real_fn(), 1);
}

#[kani::proof]
#[kani::use_stub_set(set_two)]
fn check_harness_uses_set_two() {
    assert_eq!(real_fn(), 2);
}
