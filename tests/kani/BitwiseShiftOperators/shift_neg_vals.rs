// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Ensure that shifting negative values yields the expected results.

#[kani::proof]
#[kani::unwind(5)]
fn sheck_shl() {
    let val: i32 = kani::any();
    let dist: u8 = kani::any();
    kani::assume(dist < 32);
    assert_eq!(val << dist, val.wrapping_mul(2_i32.wrapping_pow(dist.into())));
}

#[kani::proof]
#[kani::unwind(5)]
fn check_shr() {
    let val: i32 = kani::any();
    let dist: u8 = kani::any();
    kani::assume(dist < 32);
    let result = (val as i64).div_euclid(2_i64.pow(dist.into()));
    assert_eq!(val >> dist, result as i32);
}
