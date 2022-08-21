// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! One of 3 sub packages used to test specifying packages with -p
//! flag.

fn main() {
    println!("Hello, world!");
}

#[kani::proof]
fn check_bincrate_proof() {
    assert!(1 == 1);
}
