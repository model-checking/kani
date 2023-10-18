// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Ensure Kani override doesn't result in extra warnings which could block compilation when
//! users have strict lints.

#![deny(unused_variables)]

#[kani::proof]
pub fn check_panic_format() {
    let val: bool = kani::any();
    panic!("Invalid value {val}");
}

#[kani::proof]
pub fn check_panic_format_expr() {
    let val: bool = kani::any();
    panic!("Invalid value {}", !val);
}
