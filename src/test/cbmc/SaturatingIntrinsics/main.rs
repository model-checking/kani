// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]
use std::intrinsics;

include!("../../rmc-prelude.rs");

macro_rules! test_saturating_intrinsics {
    ($ty:ty) => {
        let v: $ty = __nondet();
        let w: $ty = __nondet();
        let result = intrinsics::saturating_add(v, w);
        match (0 <= v, 0 <= w) {
            (true, true) => {
                assert!(v <= result);
                assert!(w <= result);
            }
            (true, false) => {
                assert!(result == v + w);
                assert!(result <= v);
                assert!(w <= result);
            }
            (false, true) => {
                assert!(result == v + w);
                assert!(v <= result);
                assert!(result <= w);
            }
            (false, false) => {
                assert!(result <= v);
                assert!(result <= w);
            }
        }

        let result = intrinsics::saturating_sub(v, w);
        match (0 <= v, 0 <= w) {
            (true, true) => {
                assert!(result <= v);
            }
            (true, false) => {
                assert!(v <= result);
                assert!(w <= result);
            }
            (false, true) => {
                assert!(result <= v);
                assert!(result <= w);
            }
            (false, false) => {
                assert!(v <= result);
            }
        }
    };
}

fn main() {
    test_saturating_intrinsics!(u8);
    test_saturating_intrinsics!(u16);
    test_saturating_intrinsics!(u32);
    test_saturating_intrinsics!(u64);
    test_saturating_intrinsics!(usize);
    test_saturating_intrinsics!(i8);
    test_saturating_intrinsics!(i16);
    test_saturating_intrinsics!(i32);
    test_saturating_intrinsics!(i64);
    test_saturating_intrinsics!(isize);
}
