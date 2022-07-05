// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// size_of is not supported yet:
#[kani::proof]
fn assert_fndef_zst() {
    assert_eq!(std::mem::size_of_val(&h), 0);
}
