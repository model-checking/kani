// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This tests stubbing for foreign functions and methods.

use other_crate;

fn the_answer() -> u32 {
    42
}

// TODO: Split up these assertions into separate harnesses, once stubbing is able to support that.
// <https://github.com/model-checking/kani/issues/1861>
#[kani::proof]
#[kani::stub(other_crate::pub_fn, the_answer)]
#[kani::stub(other_crate::priv_fn, other_crate::the_answer)]
#[kani::stub(other_crate::pub_mod::pub_fn, the_answer)]
#[kani::stub(other_crate::pub_mod::priv_mod::pub_fn, other_crate::pub_mod::priv_mod::the_answer)]
#[kani::stub(other_crate::PubType::pub_fn, other_crate::PubType::the_answer)]
#[kani::stub(other_crate::PubType::priv_fn, other_crate::PubType::the_answer)]
#[kani::stub(other_crate::PrivType::priv_fn, other_crate::PrivType::the_answer)]
fn main() {
    // Test function stubbing
    assert_eq!(other_crate::pub_fn(), 42);
    assert_eq!(other_crate::fn_delegating_to_priv_fn(), 42);
    assert_eq!(other_crate::pub_mod::pub_fn(), 42);
    assert_eq!(other_crate::pub_mod::fn_delegating_to_priv_fn(), 42);

    // Test method stubbing
    assert_eq!(other_crate::PubType::new().pub_fn(), 42);
    assert_eq!(other_crate::PubType::new().fn_delegating_to_priv_fn(), 42);
    assert_eq!(other_crate::PubType::new().fn_delegating_to_priv_type(), 42);
}
