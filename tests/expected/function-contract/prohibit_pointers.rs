// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

struct HidesAPointer(*mut u32);

#[kani::ensures(true)]
fn hidden_pointer(h: HidesAPointer) {}

#[kani::proof_for_contract(hidden_pointer)]
fn harness() {}

#[kani::ensures(true)]
fn plain_pointer(t: *mut i32) {}

#[kani::proof_for_contract(plain_pointer)]
fn plain_ptr_harness() {}

#[kani::ensures(true)]
fn return_pointer() -> *const usize {
    unreachable!()
}

#[kani::proof_for_contract(return_pointer)]
fn return_ptr_harness() {}

#[kani::ensures(true)]
fn allowed_pointer(t: *const bool) {}

#[kani::proof_for_contract(allowed_pointer)]
fn allowed_pointer_harness() {}

#[kani::ensures(true)]
fn allowed_mut_ref(t: &mut bool) {}

#[kani::proof_for_contract(allowed_mut_ref)]
fn allowed_mut_ref_harness() {}

#[kani::ensures(true)]
fn allowed_ref(t: &bool) {}

#[kani::proof_for_contract(allowed_ref)]
fn allowed_ref_harness() {}

#[kani::ensures(true)]
fn allowed_mut_return_ref<'a>() -> &'a mut bool {
    unreachable!()
}

#[kani::proof_for_contract(allowed_mut_return_ref)]
fn allowed_mut_return_ref_harness() {}
