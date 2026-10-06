// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check if loop assign clause can be infered for inner-loop when there are local variables of outter-loop body.
//! Also checks that the `move_storagelive_call_to_loophead` liveness gate doesn't wrongly refuse copies needed for nested-loop assigns inference: checked-arithmetic initializers, call-hoisted locals, and `&mut`-argument writes.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

fn sum_pair(x: u32, y: u32) -> u32 {
    x + y
}

#[kani::proof]
fn main() {
    let mut i: u32 = 0;
    let mut s: u32 = 0;
    let t1 = kani::any_where(|x| *x < 5);
    let t2 = kani::any_where(|x| *x < 5);
    #[kani::loop_invariant(i <= 5 && s == i * 20)]
    while i < 5 {
        let mut j = sum_pair(t1, t2);
        let mut k = sum_pair(t2, t1);
        #[kani::loop_invariant(j <= 10 && k ==j)]
        while j < 10 {
            j = j + 1;
            k = k + 1;
        }
        s = s + j + k;
        i = i + 1;
    }
    assert!(s == 100);
}

#[inline(never)]
fn sum_pair2(x: u32, y: u32) -> u32 {
    x.wrapping_add(y)
}

// A checked-arithmetic argument (`t1 + 1` lowers to `Rvalue::CheckedBinaryOp` plus an
// `Assert`) must not make the group unanalyzable.
#[kani::proof]
fn checked_arith_init() {
    let mut i: u32 = 0;
    let mut s: u32 = 0;
    let t1: u32 = kani::any_where(|x: &u32| *x < 5);
    let t2: u32 = kani::any_where(|x: &u32| *x < 5);
    #[kani::loop_invariant(i <= 5)]
    while i < 5 {
        let mut j = sum_pair2(t1 + 1, t2);
        #[kani::loop_invariant(j <= 10)]
        while j < 10 {
            j = j + 1;
        }
        s = s.wrapping_add(j);
        i = i + 1;
    }
    assert!(i == 5);
}

// A call-initialized local (`a`) gets no StorageLive — its storage spans the
// whole function — so the fallback must not classify it loop-declared and
// refuse the group that reads it.
#[kani::proof]
fn init_from_call_hoisted_local() {
    let mut i: u32 = 0;
    let mut s: u32 = 0;
    let t1: u32 = kani::any_where(|x: &u32| *x < 5);
    #[kani::loop_invariant(i <= 5)]
    while i < 5 {
        let a = sum_pair2(t1, 0);
        let mut j = sum_pair2(a, 0);
        #[kani::loop_invariant(j <= 10)]
        while j < 10 {
            j = j + 1;
        }
        s = s.wrapping_add(j);
        i = i + 1;
    }
    assert!(i == 5);
}

// A local the assign hoist itself makes live at the head (`r`) must count as
// live for a later group that reads through it; `a` needs no tracking — with
// no StorageLive its storage spans the whole function.
#[kani::proof]
fn init_via_hoisted_ref() {
    let mut i: u32 = 0;
    let mut s: u32 = 0;
    let t1: u32 = kani::any_where(|x: &u32| *x < 5);
    #[kani::loop_invariant(i <= 5)]
    while i < 5 {
        let a = sum_pair2(t1, 0);
        let r = &a;
        let mut j = sum_pair2(*r, 0);
        #[kani::loop_invariant(j <= 10)]
        while j < 10 {
            j = j + 1;
        }
        s = s.wrapping_add(j);
        i = i + 1;
    }
    assert!(i == 5);
}

// A write through a `&mut` argument (`*acc = ..`) is not a definition of `acc`: the
// argument (which has no `StorageLive`) stays head-live and the group reading `*acc`
// is still copied.
fn accumulate(acc: &mut u32, t1: u32) {
    let mut i: u32 = 0;
    #[kani::loop_invariant(i <= 5)]
    while i < 5 {
        let mut j = sum_pair2(t1, *acc & 0);
        #[kani::loop_invariant(j <= 10)]
        while j < 10 {
            j = j + 1;
        }
        *acc = j & 0;
        i = i + 1;
    }
    assert!(i == 5);
}

#[kani::proof]
fn proj_write_arg_stays_live() {
    let mut acc: u32 = 0;
    accumulate(&mut acc, kani::any_where(|x: &u32| *x < 5));
}
