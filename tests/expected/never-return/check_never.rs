// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --default-unwind 11

//! Check that we can verify code inside functions that never return.
//! See https://github.com/model-checking/kani/issues/648 for more detail.

#![feature(never_type)]
pub fn found_zero() -> ! {
    panic!("Found zero");
}

pub fn found_one() -> ! {
    panic!("Found one");
}

#[kani::proof]
#[kani::unwind(11)]
fn check_never_return() {
    let mut counter: u8 = kani::any();
    kani::assume(counter < 10);

    loop {
        if counter == 0 {
            found_zero();
        }
        if counter == 1 {
            found_one();
        }
        counter -= 1;
    }
}
