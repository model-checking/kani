// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Two proof harnesses: one fails, one passes. `sort_harnesses_by_loc` processes
// later-appearing harnesses first, so `z_passes` (below) runs before
// `a_fails`. Run sequentially (the default), the pass therefore completes before the
// failure triggers the `--fail-fast` abort -- the exact situation in which the
// previous code discarded the already-completed pass (#4729).

#[kani::proof]
fn a_fails() {
    assert!(false, "intentional failure for the --fail-fast regression test");
}

#[kani::proof]
fn z_passes() {
    assert!(1 + 1 == 2);
}
