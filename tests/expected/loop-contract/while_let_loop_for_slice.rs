// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check if while-let invariant is correctly applied.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn trim_ascii_start() {
    let mut a: [u8; 10] = kani::any();
    let s = a.as_slice();
    let mut bytes = s;
    #[kani::loop_invariant(
            bytes.len() <= s.len() &&
            (bytes.len() == 0 || (&s[s.len()-bytes.len()..]).as_ptr() == bytes.as_ptr())
    )]
    while let [first, rest @ ..] = bytes {
        if first.is_ascii_whitespace() {
            bytes = rest;
        } else {
            break;
        }
    }
}
