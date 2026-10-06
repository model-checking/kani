// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that the `#[kani::loop_modifies]` clause of a loop is applied to that loop when there is
//! a loop without a loop contract between the clause and the loop, or around the loop, and when
//! the attributes are written with other paths. Each loop with a clause writes `bad`, which is
//! not in its clause, so the assigns check for `bad` must fail. If the clause was dropped, CBMC
//! would infer the write set of the loop instead.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

extern crate kani;

mod m {
    pub use kani::loop_invariant;
    pub use kani::loop_modifies;
}

/// The clause is written before the invariant of a `for` loop whose iterable expression contains
/// a loop.
#[kani::proof]
fn loop_in_iterable_expression() {
    let mut s: u32 = 0;
    let mut bad: u8 = 0;
    #[kani::loop_modifies(&s)]
    #[kani::loop_invariant(s <= kani::index as u32 * 255)]
    for x in {
        let mut n: u8 = 0;
        while n < 2 {
            n += 1;
        }
        [n, 1u8]
    } {
        s += x as u32;
        bad = 1;
    }
    let _ = bad;
}

/// The clause is written before an invariant whose `on_entry` expression contains a loop.
#[kani::proof]
fn loop_in_on_entry() {
    let mut i: u8 = 0;
    let mut bad: u8 = 0;
    #[kani::loop_modifies(&i)]
    #[kani::loop_invariant(i <= 2 && on_entry({
        let mut n: u8 = 0;
        while n < 2 {
            n += 1;
        }
        n
    }) == 2)]
    while i < 2 {
        bad = 1;
        i += 1;
    }
    let _ = bad;
}

/// The loop is the first statement of the body of a `loop` without a loop contract, so its clause
/// is assigned in the head of that loop. Kani also reports `Check assigns clause inclusion` and
/// `unwinding assertion loop 0` for this harness, even with a complete clause, so only the check
/// for `bad` is tested here.
#[kani::proof]
fn first_in_loop_without_contract() {
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    let mut bad: u8 = 0;
    loop {
        #[kani::loop_invariant(j <= 2)]
        #[kani::loop_modifies(&j)]
        while j < 2 {
            j += 1;
            bad = bad.wrapping_add(1);
        }
        i += 1;
        j = 0;
        if i >= 2 {
            break;
        }
    }
}

/// A `#[kani::loop_decreases]` expression with a loop is between the clause and the loop.
#[kani::proof]
fn loop_in_decreases() {
    let mut i: u8 = 0;
    let mut bad: u8 = 0;
    #[kani::loop_invariant(i <= 2)]
    #[kani::loop_decreases({
        let mut k: u8 = 0;
        while k < 1 {
            k += 1;
        }
        2 - i + k - 1
    })]
    #[kani::loop_modifies(&i)]
    while i < 2 {
        i += 1;
        bad = 1;
    }
    let _ = bad;
}

/// The invariant is written with another path, after the clause.
#[kani::proof]
fn other_invariant_path() {
    let mut i: u8 = 0;
    let mut bad: u8 = 0;
    #[kani::loop_modifies(&i)]
    #[m::loop_invariant(i <= 2)]
    while i < 2 {
        i += 1;
        bad = 1;
    }
    let _ = bad;
}

/// The clause is written with another path, after the invariant.
#[kani::proof]
fn other_modifies_path() {
    let mut i: u8 = 0;
    let mut bad: u8 = 0;
    #[kani::loop_invariant(i <= 2)]
    #[m::loop_modifies(&i)]
    while i < 2 {
        i += 1;
        bad = 1;
    }
    let _ = bad;
}
