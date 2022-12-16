// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This tests stubbing for public foreign functions.

use other_crate;

fn the_answer() -> u32 {
    42
}

// TODO: Split up these assertions into separate harnesses, once stubbing is able to support that.
// <https://github.com/model-checking/kani/issues/1861>
#[kani::proof]
#[kani::stub(other_crate::pub_fn, the_answer)]
#[kani::stub(other_crate::pub_mod::pub_fn, the_answer)]
fn main() {
    assert_eq!(other_crate::pub_fn(), 42);
    assert_eq!(other_crate::pub_mod::pub_fn(), 42);
}
