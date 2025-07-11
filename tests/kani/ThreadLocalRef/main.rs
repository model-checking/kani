// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Checks that thread locals work as intended.

use std::cell::RefCell;

thread_local! {
    static COND : bool = true;
    static COUNTER: RefCell<i32> = RefCell::new(0);
    static COMPLEX_DATA: RefCell<&'static str> = RefCell::new("before");
}

#[kani::proof]
fn test_bool() {
    COND.with(|&b| {
        assert!(b);
    });
}

#[kani::proof]
fn test_i32() {
    COUNTER.with(|c| {
        assert_eq!(*c.borrow(), 0);
        *c.borrow_mut() += 1;
    });
    COUNTER.with(|c| {
        assert_eq!(*c.borrow(), 1);
    });
}

// TODO: This test exposes a bug in CBMC 6.7.1. It should be re-enabled once a version of CBMC that
// includes https://github.com/diffblue/cbmc/pull/8678 has been released.
// #[kani::proof]
// #[kani::unwind(7)]
// fn test_complex_data() {
//     COMPLEX_DATA.with(|c| {
//         assert_eq!(*c.borrow(), "before");
//         *c.borrow_mut() = "after"
//     });
//     COMPLEX_DATA.with(|c| {
//         assert_eq!(*c.borrow(), "after");
//     });
// }
