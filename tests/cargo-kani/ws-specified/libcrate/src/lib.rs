// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! See the companion 'bincrate' for a comment about this test

#[kani::proof]
fn check_libcrate_proof() {
    assert!(1 == 2);
}
