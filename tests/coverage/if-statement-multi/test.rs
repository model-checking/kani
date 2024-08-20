// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --coverage -Zsource-coverage

//! Checks that we are covering all regions except
//!  * the `val == 42` condition
//!  * the `false` branch
//!
//! No coverage information is shown for `_other_function` because it's sliced
//! off: <https://github.com/model-checking/kani/issues/3445>

fn _other_function() {
    println!("Hello, world!");
}

fn test_cov(val: u32) -> bool {
    if val < 3 || val == 42 { true } else { false }
}

#[cfg_attr(kani, kani::proof)]
fn main() {
    let test1 = test_cov(1);
    let test2 = test_cov(2);
    assert!(test1);
    assert!(test2);
}
