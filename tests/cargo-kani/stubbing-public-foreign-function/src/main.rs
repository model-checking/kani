// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This tests stubbing for public foreign functions.

use other_crate;

fn the_answer() -> u32 {
    42
}

#[kani::proof]
#[kani::stub(other_crate::pub_fn, the_answer)]
fn check_pub_fn_stub() {
    assert_eq!(other_crate::pub_fn(), 42);
}

#[kani::proof]
#[kani::stub(other_crate::pub_mod::pub_fn, the_answer)]
fn check_pub_mod_fn_stub() {
    assert_eq!(other_crate::pub_mod::pub_fn(), 42);
}
