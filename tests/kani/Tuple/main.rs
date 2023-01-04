// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Tests for arbitrary tuples. Kani Lib supports up to size 12, so
//! minimum (size 1) and maximum (size 12) are tested here.

#[kani::proof]
fn test_tuple_size_1() {
    let tuple1: (usize,) = kani::any();
    kani::assume(tuple1.0 < 10);

    assert!(tuple1.0 <= 9)
}

#[kani::proof]
fn test_tuple_size_12() {
    let tuple12: (u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8) = kani::any();

    let mut sum = 0u16;
    sum += tuple12.0 as u16;
    sum += tuple12.1 as u16;
    sum += tuple12.2 as u16;
    sum += tuple12.3 as u16;
    sum += tuple12.4 as u16;
    sum += tuple12.5 as u16;
    sum += tuple12.6 as u16;
    sum += tuple12.7 as u16;
    sum += tuple12.8 as u16;
    sum += tuple12.9 as u16;
    sum += tuple12.10 as u16;
    sum += tuple12.11 as u16;

    assert!(sum <= u8::MAX as u16 * 12);
}
