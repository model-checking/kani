// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test is to check Kani's error handling of invalid usages of the `proof` harness.
// We also ensure that all errors and warnings are printed in one compilation.

#[kani::proof]
#[kani::proof]
fn multiple_proof_annotations() {}

#[kani::proof]
fn proof_with_arg(arg: bool) {
    assert!(arg);
}

#[kani::proof]
fn generic_harness<T: Default>() {
    let _ = T::default();
}
