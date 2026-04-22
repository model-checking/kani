// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
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

#[kani::proof]
#[kani::stub(local_fn, the_answer)]
fn check_local_fn_stub() {
    assert_eq!(local_fn(), 42);
}

#[kani::proof]
#[kani::stub(local_mod::pub_fn, the_answer)]
fn check_pub_fn_in_module_stub() {
    assert_eq!(local_mod::pub_fn(), 42);
}
