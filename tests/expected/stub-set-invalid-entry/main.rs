// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z stubbing

//! Test that unknown entries in a stub set are detected and reported.

fn real_fn() -> u32 {
    0
}
fn mock_fn() -> u32 {
    1
}

kani::stub_set!(bad_set, unknown_entry(real_fn, mock_fn),);

#[kani::proof]
#[kani::use_stub_set(bad_set)]
fn check_invalid_entry() {
    assert_eq!(real_fn(), 1);
}
