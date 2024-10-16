// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks -Z mem-predicates
//! Check that Kani can identify invalid value when using `can_dereference` API.

#[kani::proof]
fn check_can_dereference_char() {
    let val: [u32; 2] = kani::any();
    kani::cover!(kani::mem::can_dereference(&val as *const _ as *const [char; 2]));
    kani::cover!(!kani::mem::can_dereference(&val as *const _ as *const [char; 2]));
}

#[kani::proof]
fn check_can_dereference_always_valid() {
    let val: [char; 2] = [kani::any(), kani::any()];
    assert!(kani::mem::can_dereference(&val as *const _ as *const [u32; 2]));
}

#[kani::proof]
fn check_can_dereference_always_invalid() {
    let val: [u8; 2] = kani::any();
    kani::assume(val[0] > 1 || val[1] > 1);
    assert!(!kani::mem::can_dereference(&val as *const _ as *const [bool; 2]));
}
