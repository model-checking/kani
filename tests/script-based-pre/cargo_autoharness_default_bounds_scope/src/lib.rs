// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the default bounds of `autoharness` (`--default-unwind 20`, `--harness-timeout 60s`)
// only apply to the harnesses it generates, and not to the manual harnesses it also verifies,
// c.f. https://github.com/model-checking/kani/issues/5103.
//
// A loop of N iterations needs an unwinding bound of N + 1 to pass its unwinding assertion.

/// Automatic harness: needs a bound of 11, so it passes under the autoharness default of 20.
fn loop_10_iterations() -> u32 {
    let mut i = 0;
    while i < 10 {
        i += 1;
    }
    i
}

/// Automatic harness: needs a bound of 31, so it fails under the autoharness default of 20.
fn loop_30_iterations() -> u32 {
    let mut i = 0;
    while i < 30 {
        i += 1;
    }
    i
}

/// Manual harness without `#[kani::unwind]`: CBMC unwinds the constant-bounded loop on its own, as
/// under plain `cargo kani`. Before #5103 was fixed, it inherited the autoharness default of 20.
#[kani::proof]
fn manual_loop_30() {
    let mut i = 0;
    while i < 30 {
        i += 1;
    }
    assert_eq!(i, 30);
}

/// Manual harness whose `#[kani::unwind]` suffices; it beats any `--default-unwind`.
#[kani::proof]
#[kani::unwind(31)]
fn manual_loop_30_unwind_31() {
    let mut i = 0;
    while i < 30 {
        i += 1;
    }
    assert_eq!(i, 30);
}

/// Manual harness whose `#[kani::unwind]` is too small; it beats any `--default-unwind`.
#[kani::proof]
#[kani::unwind(5)]
fn manual_loop_30_unwind_5() {
    let mut i = 0;
    while i < 30 {
        i += 1;
    }
    assert_eq!(i, 30);
}
