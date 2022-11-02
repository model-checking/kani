// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness main --enable-unstable --enable-stubbing
//
//! This tests stubbing for local functions and methods.

fn local_fn() -> u32 {
    0
}

fn the_answer() -> u32 {
    42
}

mod local_mod {
    pub fn pub_fn() -> u32 {
        0
    }

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

struct LocalType {}

impl LocalType {
    pub fn new() -> Self {
        Self {}
    }

    pub fn pub_fn(&self) -> u32 {
        0
    }

    pub fn fn_delegating_to_priv_fn(&self) -> u32 {
        self.priv_fn()
    }

    fn priv_fn(&self) -> u32 {
        0
    }

    fn the_answer(&self) -> u32 {
        42
    }
}

// TODO: Split up these assertions into separate harnesses, once stubbing is able to support that.
// <https://github.com/model-checking/kani/issues/1861>
#[kani::proof]
#[kani::stub(local_fn, the_answer)]
#[kani::stub(local_mod::pub_fn, the_answer)]
#[kani::stub(local_mod::priv_fn, local_mod::the_answer)]
#[kani::stub(LocalType::pub_fn, LocalType::the_answer)]
#[kani::stub(LocalType::priv_fn, LocalType::the_answer)]
fn main() {
    // Test function stubbing
    assert_eq!(local_fn(), 42);
    assert_eq!(local_mod::pub_fn(), 42);
    assert_eq!(local_mod::fn_delegating_to_priv_fn(), 42);

    // Test method stubbing
    assert_eq!(LocalType::new().pub_fn(), 42);
    assert_eq!(LocalType::new().fn_delegating_to_priv_fn(), 42);
}
