// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness harness --enable-unstable --enable-stubbing
//
//! Check support for stubbing out extern C functions.

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

#[kani::proof]
#[kani::stub(libc::strlen, stubs::strlen)]
#[kani::stub(libc::sysconf, stubs::sysconf)]
fn harness() {
    let str: Box<i8> = Box::new(4);
    let str_ptr: *const i8 = &*str;
    assert_eq!(unsafe { libc::strlen(str_ptr) }, 4);
    assert_eq!(unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize, 10);
}
