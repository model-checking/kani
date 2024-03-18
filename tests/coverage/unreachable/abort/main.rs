// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test that the abort() function is respected and nothing beyond it will execute.

use std::process;

#[cfg_attr(kani, kani::proof, kani::unwind(5))]
fn main() {
    for i in 0..4 {
        if i == 1 {
            // This comes first and it should be reachable.
            process::abort();
        }
        if i == 2 {
            // This should never happen.
            process::exit(1);
        }
    }
    assert!(false, "This is unreachable");
}
