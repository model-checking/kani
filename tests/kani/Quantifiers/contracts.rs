// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]
extern crate kani;
use kani::kani_forall;

#[kani::requires(i==0)]
#[kani::ensures(|ret| {
    unsafe{
    let ptr = arr.as_ptr(); kani::forall!(| k in (0, 8)| *ptr.wrapping_byte_offset(k as isize) == 0)}})]
#[kani::modifies(arr)]
pub fn set_zero(arr: &mut [u8; 8], mut i: usize) -> usize {
    while i < 8 {
        arr[i] = 0;
        i = i + 1;
    }
    i
}

#[kani::proof_for_contract(set_zero)]
fn set_zero_harness() {
    let mut arr: [u8; 8] = kani::any();
    let i: usize = 0;
    let _j = set_zero(&mut arr, i);
}

#[kani::ensures(|ret| {
    unsafe{
    let ptr_x = xs.as_ptr();
    let ptr_y = ys.as_ptr(); kani::forall!(| k in (0, 8)| *ptr_x.wrapping_byte_offset(k as isize) == *ptr_y.wrapping_byte_offset(k as isize))}})]
#[kani::modifies(ys)]
pub fn copy(xs: &mut [u8; 8], ys: &mut [u8; 8]) {
    let mut i = 0;
    while i < 8 {
        ys[i] = xs[i];
        i = i + 1;
    }
}

#[kani::proof_for_contract(copy)]
fn copy_harness() {
    let mut xs: [u8; 8] = kani::any();
    let mut ys: [u8; 8] = kani::any();
    copy(&mut xs, &mut ys);
}
