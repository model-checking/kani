// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test that the abort() function is respected and nothing beyond it will execute.

use std::process;

#[kani::proof]
fn main() {
    for i in 0..4 {
        if i == 1 {
            kani::cover!();
            // This comes first and it should be reachable.
            process::abort();
        }
        if i == 2 {
            kani::cover!();
            // This should never happen.
            process::exit(1);
        }
    }
    assert!(false, "This is unreachable");
}
