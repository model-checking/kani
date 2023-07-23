// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test is used to check that an invocation of `kani` or `cargo kani`
//! prints the version and invocation type as expected.

#[kani::proof]
fn dummy() {
    assert!(1 + 1 == 2);
}

