// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! We don't use any of our dependencies to keep the test fast

#[kani::proof]
fn check_u8_u32() {
    let before: u8 = kani::any();
    let temp = before as u32;
    let after: u8 = temp.try_into().unwrap();
    assert_eq!(after, before);
}

#[kani::proof]
fn check_u8_i16() {
    let before: u8 = kani::any();
    let temp = before as i16;
    let after: u8 = temp.try_into().unwrap();
    assert_eq!(after, before);
}
