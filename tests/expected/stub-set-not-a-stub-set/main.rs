// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z stubbing

//! Test that using a non-stub-set item in `#[kani::use_stub_set(...)]` is
//! detected and reported as an error. `NOT_A_STUB_SET` is a regular const,
//! not a `kani::stub_set!` definition, so it should fail resolution.

fn real_fn() -> u32 {
    0
}
fn mock_fn() -> u32 {
    1
}

const NOT_A_STUB_SET: () = ();

#[kani::proof]
#[kani::use_stub_set(NOT_A_STUB_SET)]
fn check_not_a_stub_set() {
    assert_eq!(real_fn(), 1);
    let _ = mock_fn();
}
