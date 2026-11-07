// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

pub fn count_down(mut x: u8) -> u8 {
    #[kani::loop_invariant(true)]
    #[kani::loop_decreases(x)]
    while x > 0 {
        x -= 1;
    }
    x
}
