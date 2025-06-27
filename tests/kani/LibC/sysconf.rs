// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Check support for `sysconf`.

#![feature(rustc_private)]
extern crate libc;

#[kani::proof]
fn main() {
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
}
