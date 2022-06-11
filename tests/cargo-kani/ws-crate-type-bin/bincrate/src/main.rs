// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test's purpose is to ensure that we successfully build
//! a crate with a dependency, when this crate's crate-type=bin.
//! (i.e. we're in a main.rs, not lib.rs)

//! Previously, this would fail because we didn't generate an 'rlib'
//! for the dependency, and for 'bin' cargo would try to link against 'rlib'
//! instead of 'rmeta'.

use libcrate; // critical to trigger bug

fn main() {
    println!("Hello, world!");
}

#[kani::proof]
fn check_bincrate_proof() {
    assert!(1 == 1);
}
