// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! One of 2 sub packages used to test specifying packages with -p
//! flag.

#[kani::proof]
fn check_libcrate_proof() {
    assert!(1 == 2);
}
