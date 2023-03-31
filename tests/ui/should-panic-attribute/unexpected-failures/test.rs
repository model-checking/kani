// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that verfication fails when `#[kani::should_panic]` is used but not
//! all failures encountered are panics.

fn trigger_overflow() {
    let x: u32 = kani::any();
    let _ = 42 << x;
}

#[kani::proof]
#[kani::should_panic]
fn check() {
    match kani::any() {
        0 => panic!("panicked on the `0` arm!"),
        1 => panic!("panicked on the `1` arm!"),
        _ => {
            trigger_overflow();
            ()
        }
    }
}
