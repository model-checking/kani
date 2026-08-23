// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// A crate with exactly one real proof harness. The test drives it with a
// `--harness` filter that matches nothing, so verification never runs.

#[kani::proof]
fn existing_harness() {
    assert!(1 + 1 == 2);
}
