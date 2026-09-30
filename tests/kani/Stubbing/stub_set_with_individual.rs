// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! Test combining use_stub_set with individual stub attributes.

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

kani::stub_set!(my_set, stub(real_a, stub_a),);

#[kani::proof]
#[kani::use_stub_set(my_set)]
#[kani::stub(real_b, stub_b)]
fn check_stub_set_with_individual_stub() {
    assert_eq!(real_a(), 1);
    assert_eq!(real_b(), 2);
}
