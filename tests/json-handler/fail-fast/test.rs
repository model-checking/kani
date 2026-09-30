// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// sort_harnesses_by_loc runs later-appearing harnesses first: z_passes, then a_fails, then
// a0_never_runs.

#[kani::proof]
fn a0_never_runs() {
    assert!(true);
}

#[kani::proof]
fn a_fails() {
    assert!(false, "intentional failure for the --fail-fast regression test");
}

#[kani::proof]
fn z_passes() {
    assert!(1 + 1 == 2);
}
