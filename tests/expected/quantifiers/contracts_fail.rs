// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

/// Copy only first 7 elements and left the last one unchanged.
#[kani::ensures(|ret| { unsafe{
    let ptr_x = xs.as_ptr();
    let ptr_y = ys.as_ptr();
    kani::forall!(| k in (0, 8)| *ptr_x.wrapping_byte_offset(k as isize) == *ptr_y.wrapping_byte_offset(k as isize))}})]
#[kani::modifies(ys)]
pub fn copy(xs: &mut [u8; 8], ys: &mut [u8; 8]) {
    let mut i = 0;
    while i < 7 {
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
