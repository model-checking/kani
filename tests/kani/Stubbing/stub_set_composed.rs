// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! Test composing stub sets with use_stub_set().

fn real_a() -> u32 {
    0
}

fn real_b() -> u32 {
    0
}

fn real_c() -> u32 {
    0
}

fn stub_a() -> u32 {
    1
}

fn stub_b() -> u32 {
    2
}

fn stub_c() -> u32 {
    3
}

kani::stub_set!(set_a, stub(real_a, stub_a),);

kani::stub_set!(set_b, stub(real_b, stub_b),);

kani::stub_set!(combined, use_stub_set(set_a), use_stub_set(set_b), stub(real_c, stub_c),);

#[kani::proof]
#[kani::use_stub_set(combined)]
fn check_composed_stub_sets() {
    assert_eq!(real_a(), 1);
    assert_eq!(real_b(), 2);
    assert_eq!(real_c(), 3);
}
