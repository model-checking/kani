// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts -Z quantifiers

//! Check that the decreases clause of an outer loop is checked on that loop when the function
//! also has a quantifier, whose closure may be generated while the function is generated.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn nested_after_quantifier_harness() {
    let a: [u8; 4] = kani::any();
    kani::assume(kani::forall!(|k in (0, 4)| a[k] < 100));
    let n: u8 = kani::any();
    let mut x: u8 = 5;
    // Bug: `x` is reset to 5 at each iteration, and the loop does not terminate.
    #[kani::loop_decreases(x)]
    #[kani::loop_invariant(x <= 5)]
    while n < 10 {
        #[kani::loop_invariant(x <= 5)]
        while x > 0 {
            x -= 1;
        }
        x = 5;
    }
}
