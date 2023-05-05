// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Minimized from vmm-sys-util/src/linux/aio.rs new()
#![feature(c_variadic)]

#[allow(non_camel_case_types)]
type c_long = u64;

pub unsafe extern "C" fn my_add(num: c_long, mut args: ...) -> c_long {
    let mut accum: c_long = 0;
    for i in 0..num {
        accum += args.arg::<c_long>()
    }
    accum
}

#[kani::proof]
fn main() {
    let arg0: c_long = 2;
    let arg1: c_long = 4;
    let x = unsafe { my_add(0, arg0, arg1) };
    assert!(x == 0);

    let x = unsafe { my_add(2, arg0, arg1) };
    assert!(x == 6);
}
