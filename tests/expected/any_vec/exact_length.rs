// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test `exact_vec` API
#[kani::proof]
fn check_access_length_zero() {
    let data = kani::vec::exact_vec::<u8, 0>();
    assert_eq!(data.len(), 0);
    assert_eq!(data.capacity(), data.len());
    let val = unsafe { *data.get_unchecked(0) };
    kani::cover!(val == 0);
}

#[derive(kani::Arbitrary, Copy, Clone)]
struct Dummy(i32, u8);

#[kani::proof]
fn check_access_length_17() {
    let data = kani::vec::exact_vec::<Dummy, 17>();
    assert_eq!(data.len(), 17);
    assert_eq!(data.capacity(), data.len());

    let val = unsafe { *data.get_unchecked(17) };
    kani::cover!(val.0 == 0);
}
