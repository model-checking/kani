// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check the length of a constant slice of a zero-sized type. Computing it from the size of the
//! allocation used to divide by zero in the compiler.

const Z: &[()] = &[(), (), ()];
const Z_SPLIT_AT: &[()] = Z.split_at(1).1;

#[kani::proof]
fn check_zst_slice() {
    assert_eq!(Z.len(), 3);
    assert_eq!(Z_SPLIT_AT.len(), 2);
}
