// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks

//! A store through a pointer to a type without validity requirements writes bytes, not a value of
//! the memory's type: reading them back as that type is checked, c.f.
//! <https://github.com/model-checking/kani/issues/5021>. The write and the read are in the same
//! basic block, which the checks used to instrument only up to its first check.

#[kani::proof]
fn via_const_ptr() {
    let mut b = true;
    let p = &mut b as *mut bool as *const u8 as *mut u8;
    unsafe { *p = 2 };
    let _x = b; // UB: reads an invalid bool
}

#[kani::proof]
fn via_transmuted_ref() {
    let mut b = true;
    let r: &mut u8 = unsafe { std::mem::transmute::<&mut bool, &mut u8>(&mut b) };
    *r = 2;
    let _x = b; // UB: reads an invalid bool
}

#[kani::proof]
fn valid_byte() {
    let mut b = true;
    let p = &mut b as *mut bool as *mut u8;
    unsafe { *p = 0 };
    assert!(!b);
}
