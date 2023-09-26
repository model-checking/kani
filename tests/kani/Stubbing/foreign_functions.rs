// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! Check support for stubbing out foreign functions.

#![feature(rustc_private)]
extern crate libc;

use libc::c_char;
use libc::c_int;
use libc::c_longlong;
use libc::size_t;

#[allow(dead_code)] // Avoid warning when using stubs.
#[allow(unused_variables)]
mod stubs {
    use super::*;

    pub unsafe extern "C" fn strlen(cs: *const c_char) -> size_t {
        4
    }

    pub unsafe extern "C" fn sysconf(_input: c_int) -> c_longlong {
        10
    }
}

fn dig_deeper(input: c_int) {
    unsafe {
        type FunctionPointerType = unsafe extern "C" fn(c_int) -> c_longlong;
        let ptr: FunctionPointerType = libc::sysconf;
        assert_eq!(ptr(input) as usize, 10);
    }
}

fn deeper_call() {
    dig_deeper(libc::_SC_PAGESIZE)
}

fn function_pointer_call(function_pointer: unsafe extern "C" fn(c_int) -> c_longlong) {
    assert_eq!(unsafe { function_pointer(libc::_SC_PAGESIZE) } as usize, 10);
}

#[kani::proof]
#[kani::stub(libc::strlen, stubs::strlen)]
fn standard() {
    let str: Box<c_char> = Box::new(4);
    let str_ptr: *const c_char = &*str;
    assert_eq!(unsafe { libc::strlen(str_ptr) }, 4);
}

#[kani::proof]
#[kani::stub(libc::strlen, stubs::strlen)]
fn function_pointer_standard() {
    let str: Box<c_char> = Box::new(4);
    let str_ptr: *const c_char = &*str;
    let new_ptr = libc::strlen;
    assert_eq!(unsafe { new_ptr(str_ptr) }, 4);
}

#[kani::proof]
#[kani::stub(libc::sysconf, stubs::sysconf)]
fn function_pointer_with_layers() {
    deeper_call();
}

#[kani::proof]
#[kani::stub(libc::sysconf, stubs::sysconf)]
fn function_pointer_as_parameter() {
    type FunctionPointerType = unsafe extern "C" fn(c_int) -> c_longlong;
    let function_pointer: FunctionPointerType = libc::sysconf;
    function_pointer_call(function_pointer);
    function_pointer_call(libc::sysconf);
}
