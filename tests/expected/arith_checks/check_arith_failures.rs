// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that code after an overflow check failure should be unreachable.
//! Related issue: https://github.com/model-checking/kani/issues/1208
#[kani::proof]
fn check_arith_overflow() {
    let a = [0; 5];
    let ptr0: *const i32 = &a[0];
    let ptr1: *const i32 = &a[1];
    let _: usize = ptr0 as usize - ptr1 as usize;
    unreachable!("Previous statement should fail");
}
