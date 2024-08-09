// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zaliasing

#[kani::proof]
fn main() {
    let x = Box::new(10);
    let ref_x = Box::into_raw(x);
    let raw_1 = ref_x as *mut i32;
    let raw_2 = ref_x as *const i32;
    let _write = unsafe { *raw_1 = 100 };
    let _read = unsafe { *raw_2 };
    let _write = unsafe { *raw_1 = 110 };
}
