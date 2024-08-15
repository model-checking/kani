// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --coverage -Zsource-coverage

//! Checks that we print a line which points the user to the path where coverage
//! results have been saved. The line should look like:
//! ```
//! [info] Coverage results saved to /path/to/outdir/kanicov_YYYY-MM-DD_hh-mm
//! ```

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
