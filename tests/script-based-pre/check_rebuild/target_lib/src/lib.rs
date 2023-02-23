// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use num_bigint::ToBigInt;

#[kani::proof]
#[kani::unwind(2)]
fn check_convert_u8() {
    let before: u8 = kani::any();
    let big_int = before.to_bigint().unwrap();
    let after: u8 = big_int.try_into().unwrap();
    assert_eq!(after, before);

    if before != after {
        unreachable!();
    }
}

#[kani::proof]
#[kani::unwind(2)]
fn check_convert_i8() {
    let before: i8 = kani::any();
    let big_int = before.to_bigint().unwrap();
    let after: i8 = big_int.try_into().unwrap();
    assert_eq!(after, before);

    if before != after {
        unreachable!();
    }
}
