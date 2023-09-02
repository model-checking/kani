// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Zconcrete-playback --concrete-playback=print

// This test checks that Kani generates a concrete playback test for UB checks
// (e.g. dereferencing a null pointer)

#[kani::proof]
fn null_ptr() {
    let x = 42;
    let nd: i32 = kani::any();
    let ptr: *const i32 = if nd != 15 { &x as *const i32 } else { std::ptr::null() };
    let _y = unsafe { *ptr };
}
