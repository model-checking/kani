// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This tests stubbing for private foreign functions.

use other_crate;

#[kani::proof]
#[kani::stub(other_crate::priv_fn, other_crate::the_answer)]
fn check_priv_fn_stub() {
    assert_eq!(other_crate::fn_delegating_to_priv_fn(), 42);
}

#[kani::proof]
#[kani::stub(other_crate::pub_mod::priv_mod::pub_fn, other_crate::pub_mod::priv_mod::the_answer)]
fn check_priv_mod_fn_stub() {
    assert_eq!(other_crate::pub_mod::fn_delegating_to_priv_fn(), 42);
}
