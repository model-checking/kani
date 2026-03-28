// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This tests stubbing for foreign methods.

use other_crate;

#[kani::proof]
#[kani::stub(other_crate::PubType::pub_fn, other_crate::PubType::the_answer)]
fn check_pub_method_stub() {
    assert_eq!(other_crate::PubType::new().pub_fn(), 42);
}

#[kani::proof]
#[kani::stub(other_crate::PubType::priv_fn, other_crate::PubType::the_answer)]
fn check_priv_method_stub() {
    assert_eq!(other_crate::PubType::new().fn_delegating_to_priv_fn(), 42);
}

#[kani::proof]
#[kani::stub(other_crate::PrivType::priv_fn, other_crate::PrivType::the_answer)]
fn check_priv_type_method_stub() {
    assert_eq!(other_crate::PubType::new().fn_delegating_to_priv_type(), 42);
}

fn main() {}
