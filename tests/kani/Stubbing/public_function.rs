// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness main -Z stubbing
//
//! This tests stubbing for public local functions.

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

    fn the_answer() -> u32 {
        42
    }
}

// TODO: Split up these assertions into separate harnesses, once stubbing is able to support that.
// <https://github.com/model-checking/kani/issues/1861>
#[kani::proof]
#[kani::stub(local_fn, the_answer)]
#[kani::stub(local_mod::pub_fn, the_answer)]
fn main() {
    assert_eq!(local_fn(), 42);
    assert_eq!(local_mod::pub_fn(), 42);
}
