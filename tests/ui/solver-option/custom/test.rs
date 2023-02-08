// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --solver custom=kissat

//! Checks that `--solver` accepts `custom=<binary>`

#[kani::proof]
fn check_solver_option() {
    let a: [i32; 5] = kani::any();
    let s = &a[..];
    assert_eq!(a, s);
}
