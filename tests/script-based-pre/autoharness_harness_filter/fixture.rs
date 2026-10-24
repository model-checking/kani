// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Two manual harnesses and one function that autoharness selects. The test runs
// `kani autoharness` with a `--harness` filter that matches one manual harness.

fn add(a: u8, b: u8) -> u8 {
    a.wrapping_add(b)
}

#[kani::proof]
fn manual_selected() {
    assert!(add(1, 1) == 2);
}

#[kani::proof]
fn manual_other() {
    assert!(add(2, 2) == 4);
}
