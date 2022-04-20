// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that none of these operations trigger spurious overflow checks.
#![feature(core_intrinsics)]
#![feature(unchecked_math)]
use std::intrinsics::add_with_overflow;

// `checked_shr` and `checked_shl` require `u32` for their argument. We use
// `u32` in those cases and `u8` for the rest because they perform better.
macro_rules! verify_no_overflow {
    ($ty:ty, $cf: ident, $uf: ident, $fwo: ident) => {{
        let a: $ty = kani::any();
        let b: $ty = kani::any();
        let checked = a.$cf(b);
        kani::assume(checked.is_some());
        let (res, overflow) = $fwo(a, b);
        assert!(!overflow);
        assert!(checked.unwrap() == res);
    }};
}

// macro_rules! verify_overflow {
//     ($ty:ty, $cf: ident, $uf: ident) => {{
//         let a: $ty = kani::any();
//         let b: $ty = kani::any();
//         let checked = a.$cf(b);
//         let unchecked = a.$
//         kani::assume(checked.is_none());
//         let (res, overflow) = $op_with_overflow(a, b);
//         assert!(overflow);
//         assert!
//     }};
// }

#[kani::proof]
fn test_add_with_overflow() {
    verify_no_overflow!(u8, checked_add, unchecked_add, add_with_overflow);
    let unchecked = unsafe { 1u32.unchecked_add(1) };
    assert!(unchecked == 2);
}
