// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Three proof harnesses: one fails, one passes, one never runs. `sort_harnesses_by_loc` processes
// later-appearing harnesses first, so `z_passes` (below) runs before `a_fails`, and `a0_never_runs`
// (first in the file) is queued behind them. Run sequentially (the default), the pass therefore
// completes before the failure triggers the `--fail-fast` abort -- the exact situation in which the
// previous code discarded the already-completed pass (#4729) -- and `a0_never_runs` is skipped by it.

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
