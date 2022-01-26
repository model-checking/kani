// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// source: vm-sys-util src/linux/signal.rs register_signal_handler

// mock-up libc
#[allow(non_camel_case_types)]
pub enum c_int {}
#[allow(non_camel_case_types)]
pub enum c_void {}

pub type SignalHandler = extern "C" fn(num: c_int, _unused: *mut c_void) -> ();

extern "C" fn handle_signal(_: c_int, _: *mut c_void) {}

fn main() {
    let x = handle_signal as *const () as usize;
    assert!(x != 0);
}
