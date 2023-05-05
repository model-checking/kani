// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z c-ffi --c-lib tests/kani/ForeignItems/lib.c

//! Check that Kani correctly handles function pointers to C functions.
//! Note that Kani today trusts that the extern declaration is compatible with the C definition.
//! Failures to do so will result in a CBMC type mismatch.

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
