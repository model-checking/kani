// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This tests stubbing for foreign methods.

use other_crate;

// TODO: Split up these assertions into separate harnesses, once stubbing is able to support that.
// <https://github.com/model-checking/kani/issues/1861>
#[kani::proof]
#[kani::stub(other_crate::PubType::pub_fn, other_crate::PubType::the_answer)]
#[kani::stub(other_crate::PubType::priv_fn, other_crate::PubType::the_answer)]
#[kani::stub(other_crate::PrivType::priv_fn, other_crate::PrivType::the_answer)]
fn main() {
    assert_eq!(other_crate::PubType::new().pub_fn(), 42);
    assert_eq!(other_crate::PubType::new().fn_delegating_to_priv_fn(), 42);
    assert_eq!(other_crate::PubType::new().fn_delegating_to_priv_type(), 42);
}
