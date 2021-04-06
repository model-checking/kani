// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Minimized from vmm-sys-util/src/linux/aio.rs new()
#![feature(c_variadic)]

#[allow(non_camel_case_types)]
type c_long = i64;

pub unsafe extern "C" fn syscall(_num: c_long, _: ...) {}

fn main() {
    let arg0: c_long = 0;
    let arg1: c_long = 1;
    let _x = unsafe { syscall(0, arg0, arg1) };
}
