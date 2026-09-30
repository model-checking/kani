// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! Test basic stub set functionality.

fn real_a() -> u32 {
    0
}

fn real_b() -> u32 {
    0
}

fn stub_a() -> u32 {
    1
}

fn stub_b() -> u32 {
    2
}

kani::stub_set!(my_stubs, stub(real_a, stub_a), stub(real_b, stub_b),);

#[kani::proof]
#[kani::use_stub_set(my_stubs)]
fn check_stub_set_basic() {
    assert_eq!(real_a(), 1);
    assert_eq!(real_b(), 2);
}
