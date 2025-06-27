// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness main -Z stubbing
//
//! This tests stubbing for private local functions.

mod local_mod {
    pub fn fn_delegating_to_priv_fn() -> u32 {
        priv_fn()
    }

    fn priv_fn() -> u32 {
        0
    }

    fn the_answer() -> u32 {
        42
    }
}

#[kani::proof]
#[kani::stub(local_mod::priv_fn, local_mod::the_answer)]
fn main() {
    assert_eq!(local_mod::fn_delegating_to_priv_fn(), 42);
}
