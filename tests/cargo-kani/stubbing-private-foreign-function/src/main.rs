// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This tests stubbing for private foreign functions.

use other_crate;

// TODO: Split up these assertions into separate harnesses, once stubbing is able to support that.
// <https://github.com/model-checking/kani/issues/1861>
#[kani::proof]
#[kani::stub(other_crate::priv_fn, other_crate::the_answer)]
#[kani::stub(other_crate::pub_mod::priv_mod::pub_fn, other_crate::pub_mod::priv_mod::the_answer)]
fn main() {
    assert_eq!(other_crate::fn_delegating_to_priv_fn(), 42);
    assert_eq!(other_crate::pub_mod::fn_delegating_to_priv_fn(), 42);
}
