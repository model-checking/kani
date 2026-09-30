// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts -Zloop-contracts -Zquantifiers --solver z3

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

/// GCD with a quantifier-based loop invariant proving that the set of common
/// divisors is preserved across iterations. Uses a typed u64 quantifier
/// variable with modulo in the predicate body.
#[kani::requires(x > 0 && y > 0)]
#[kani::ensures(|&result| result > 0)]
fn gcd(x: u64, y: u64) -> u64 {
    let mut a = x;
    let mut b = y;
    #[kani::loop_invariant(
        a > 0
        // The `d == 0` guard is retained defensively: overflow checks in the
        // predicate body are dropped during pure expression inlining, so if
        // `x % d` were evaluated with `d = 0`, it would be undefined behavior.
        // The range (1, ...) should exclude 0, but the guard provides an extra
        // layer of safety.
        && kani::forall!(|d: u64 in (1, a.saturating_add(1))|
            d == 0 || ((x % d == 0 && y % d == 0)
            == (a % d == 0 && b % d == 0)))
    )]
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

#[kani::proof_for_contract(gcd)]
#[kani::solver(z3)]
fn check_gcd() {
    let x: u64 = kani::any();
    let y: u64 = kani::any();
    gcd(x, y);
}
