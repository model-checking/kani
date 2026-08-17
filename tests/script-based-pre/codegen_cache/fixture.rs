// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fixture for the codegen cache efficacy test.
//!
//! Deliberately re-uses the same types and the same source lines across several harnesses, which
//! is the pattern the codegen cache exists to exploit: without it, each harness re-codegens the
//! same `Ty`s and `Span`s from scratch.

#[derive(Clone, Copy, PartialEq, Eq)]
struct Pair {
    left: u32,
    right: u32,
}

fn combine(p: Pair) -> u64 {
    p.left as u64 + p.right as u64
}

fn sum_slice(xs: &[u32]) -> u64 {
    let mut total = 0u64;
    for x in xs {
        total += *x as u64;
    }
    total
}

#[kani::proof]
fn check_combine_one() {
    let p = Pair { left: kani::any(), right: kani::any() };
    kani::assume(p.left < 1000 && p.right < 1000);
    assert!(combine(p) < 2000);
}

#[kani::proof]
fn check_combine_two() {
    let p = Pair { left: kani::any(), right: kani::any() };
    kani::assume(p.left < 10 && p.right < 10);
    assert!(combine(p) < 20);
}

#[kani::proof]
fn check_sum_slice() {
    let xs: [u32; 4] = [kani::any(), kani::any(), kani::any(), kani::any()];
    kani::assume(xs.iter().all(|x| *x < 10));
    assert!(sum_slice(&xs) < 40);
}

#[kani::proof]
fn check_pair_eq() {
    let p = Pair { left: kani::any(), right: kani::any() };
    assert!(p == p);
}
