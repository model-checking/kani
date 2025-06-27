// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test is used to check that an invocation of `cargo kani` prints the version
//! and invocation type as expected.

fn main() {
    println!("Hello, world!");
}

#[kani::proof]
fn dummy() {
    assert!(1 + 1 == 2);
}
