// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check Kani handling of C-FFI function pointers when C-FFI support is disabled.

extern "C" {
    fn foreign(i: u32) -> u32;
}

fn call_on(input: u32, func: unsafe extern "C" fn(u32) -> u32) -> u32 {
    unsafe { func(input) }
}

fn may_not_call(call: bool, input: u32, func: unsafe extern "C" fn(u32) -> u32) -> Option<u32> {
    call.then(|| unsafe { func(input) })
}

#[kani::proof]
fn check_fn_ptr_called() {
    let input: u32 = kani::any();
    assert_eq!(call_on(input, foreign), input);
}

#[kani::proof]
fn check_fn_ptr_not_called() {
    let input: u32 = kani::any();
    let should_call = kani::any_where(|v| !v);
    assert_eq!(may_not_call(should_call, input, foreign), None);
}
