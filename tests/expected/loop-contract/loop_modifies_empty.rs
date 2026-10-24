// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that a `#[kani::loop_modifies]` clause without targets (empty, or with targets of zero
//! size only) verifies for a loop that writes only variables that exist only for the loop: the
//! variables declared in its body, and the index and pattern of a `for` loop.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[derive(Clone, Copy)]
struct Zst;

#[kani::proof]
fn loop_with_empty_clause() {
    #[kani::loop_invariant(true)]
    #[kani::loop_modifies()]
    loop {
        let y: u8 = kani::any();
        if y > 3 {
            break;
        }
    }
}

#[kani::proof]
fn for_loop_with_empty_clause() {
    let a: [u8; 3] = kani::any();
    #[kani::loop_invariant(true)]
    #[kani::loop_modifies()]
    for x in a {
        assert!(x as u32 <= 255);
    }
}

#[kani::proof]
fn for_loop_with_zero_sized_target() {
    let a: [u8; 3] = kani::any();
    let z = Zst;
    #[kani::loop_invariant(true)]
    #[kani::loop_modifies(&z)]
    for x in a {
        assert!(x as u32 <= 255);
    }
    let _ = z;
}
