// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that `check_assert` is fully covered. At present, the coverage for
//! this test reports an uncovered single-column region at the end of the `if`
//! statement: <https://github.com/model-checking/kani/issues/3455>

#[kani::proof]
fn check_assert() {
    let x: u32 = kani::any_where(|val| *val == 5);
    if x > 3 {
        assert!(x > 4);
    }
}
