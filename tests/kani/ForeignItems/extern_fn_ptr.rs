// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z c-ffi --c-lib tests/kani/ForeignItems/lib.c

//! Check that Kani correctly handle function pointers to C functions.

extern "C" {
    /// Returns i + 2
    fn takes_int(i: u32) -> u32;
}

fn call_on(input: u32, func: Option<unsafe extern "C" fn(u32) -> u32>) -> Option<u32> {
    func.and_then(|f| Some(unsafe { f(input) }))
}

#[kani::proof]
fn check_extern_fn_ptr() {
    let input: u32 = kani::any();
    assert_eq!(call_on(0, None), None);
    assert_eq!(call_on(input, Some(takes_int)).unwrap(), unsafe { takes_int(input) });
}
