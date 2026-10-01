// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! Check that an empty `modifies` target does not allow any write: `write_first` names the
//! empty slice `&mut a[..0]` but writes `a[0]` through its data pointer.

#[kani::modifies(x)]
fn write_first(x: &mut [u8]) {
    unsafe { *x.as_mut_ptr() = 1 };
}

#[kani::proof_for_contract(write_first)]
fn check_write_first() {
    let mut a: [u8; 4] = [0; 4];
    write_first(&mut a[..0]);
}
