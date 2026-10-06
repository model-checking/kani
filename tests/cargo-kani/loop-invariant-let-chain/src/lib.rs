// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that `#[kani::loop_invariant]` accepts `while` loops whose condition
//! is a let chain (https://github.com/model-checking/kani/issues/4943).

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn chain_basic() {
    let mut remaining: usize = kani::any();
    kani::assume(remaining <= 4);
    #[kani::loop_invariant(remaining <= 4)]
    while remaining > 0
        && let Some(next) = remaining.checked_sub(1)
    {
        remaining = next;
    }
    assert!(remaining <= 4);
}

#[kani::proof]
fn chain_multi_let() {
    let mut n: u8 = kani::any();
    kani::assume(n <= 10);
    #[kani::loop_invariant(n <= 10)]
    while n > 0
        && let Some(m) = n.checked_sub(1)
        && let Some(k) = m.checked_add(0)
    {
        n = k;
    }
    assert!(n <= 10);
}

#[kani::proof]
fn chain_short_circuits() {
    // The false first operand must prevent the scrutinee (and its side
    // effect) from ever being evaluated. The operand is opaquely false (a
    // bounded symbolic, not a literal) so nothing const-folds the loop away;
    // `n` is never written in the loop, so it is not havoced.
    let n: u8 = kani::any();
    kani::assume(n < 5);
    let mut polled = false;
    let mut src: Option<u8> = Some(1);
    #[kani::loop_invariant(!polled)]
    while n >= 5
        && let Some(_v) = {
            polled = true;
            src.take()
        }
    {}
    assert!(!polled);
}

#[kani::proof]
fn chain_labeled() {
    let mut n: u8 = kani::any();
    kani::assume(n <= 4);
    #[kani::loop_invariant(n <= 4)]
    'outer: while n > 0
        && let Some(next) = n.checked_sub(1)
    {
        n = next;
        if n == 1 {
            break 'outer;
        }
    }
    assert!(n <= 1);
}

#[kani::proof]
fn chain_cfg_attr() {
    // The cfg_attr spelling is how verify-rust-std applies the attribute
    // (the motivating case in issue 4943); a regression test that the chain
    // is detected and rewritten through that spelling.
    let mut remaining: usize = kani::any();
    kani::assume(remaining <= 4);
    #[cfg_attr(kani, kani::loop_invariant(remaining <= 4))]
    while remaining > 0
        && let Some(next) = remaining.checked_sub(1)
    {
        remaining = next;
    }
    assert!(remaining <= 4);
}

#[kani::proof]
fn chain_invariant_violated() {
    // Deliberately-failing harness: a falsifiable invariant must FAIL,
    // pinning that the contract is applied to the rewritten loop rather
    // than silently dropped.
    let mut n: u8 = kani::any();
    kani::assume(n <= 4);
    #[kani::loop_invariant(n == 4)]
    while n > 0
        && let Some(next) = n.checked_sub(1)
    {
        n = next;
    }
    assert!(n <= 4);
}

#[kani::proof]
fn chain_loop_modifies_after_invariant() {
    // Deliberately-failing harness: a `#[kani::loop_modifies]` placed after the
    // invariant must carry over to the rewritten `loop`. `k` is assigned but
    // omitted from the clause, so the assigns check must FAIL; a passing harness
    // would also pass if the attribute were silently dropped.
    let mut n: u8 = kani::any();
    kani::assume(n <= 6);
    let mut k: u8 = 0;
    #[kani::loop_invariant(n <= 6)]
    #[kani::loop_modifies(&n)]
    while n > 0
        && let Some(m) = n.checked_sub(1)
    {
        n = m;
        k = 1; // not in loop_modifies
    }
    assert!(k <= 1);
}
