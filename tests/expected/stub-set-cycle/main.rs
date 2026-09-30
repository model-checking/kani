// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z stubbing

//! Test that circular stub set references are detected and reported as errors.

fn real_fn() -> u32 {
    0
}
fn mock_fn() -> u32 {
    1
}

kani::stub_set!(set_a, stub(real_fn, mock_fn), use_stub_set(set_b),);
kani::stub_set!(set_b, use_stub_set(set_a),);

#[kani::proof]
#[kani::use_stub_set(set_a)]
fn check_cycle() {
    assert_eq!(real_fn(), 1);
}
