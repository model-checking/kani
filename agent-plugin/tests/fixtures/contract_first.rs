// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Single-file fixture for the contract-first workflow: contracts, a loop with an invariant and a
//! decreases clause (so the proof is unbounded and shows termination), and a caller that relies on
//! the callee's contract. AutoHarness verifies every contract with no hand-written harness.
#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

/// Counts down to zero. The input is unbounded: unwinding could not prove this, a loop contract can.
#[kani::requires(n <= 1_000_000_000)]
#[kani::ensures(|r: &u32| *r == 0)]
pub fn count_down(n: u32) -> u32 {
    let mut x = n;
    #[kani::loop_invariant(x <= n)]
    #[kani::loop_decreases(x)]
    while x > 0 {
        x -= 1;
    }
    x
}

#[kani::requires(a <= 1000 && b <= 1000)]
#[kani::ensures(|r: &u32| *r == a + b)]
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}

/// A caller: it uses the verified contracts of `add` and `count_down`, not their bodies.
#[kani::requires(a <= 1000 && b <= 1000)]
#[kani::ensures(|r: &u32| *r == 0)]
pub fn total_zero(a: u32, b: u32) -> u32 {
    count_down(add(a, b))
}

/// No contract yet, and a loop without an invariant: the map asks for both.
pub fn sum_to(n: u32) -> u32 {
    let mut s = 0;
    let mut i = 0;
    while i < n && i < 10 {
        s += i;
        i += 1;
    }
    s
}
