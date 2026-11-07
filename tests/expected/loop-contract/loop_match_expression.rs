// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

#![feature(stmt_expr_attributes, proc_macro_hygiene)]

// Keep the loop directly in the match arm to exercise the trailing comma.
#[rustfmt::skip]
fn count_to(n: u32, run: bool) -> u32 {
    let mut i = 0;
    match run {
        true =>
        #[kani::loop_modifies(&i)]
        #[kani::loop_invariant(i <= n)]
        loop {
            if i == n {
                return i;
            }
            i += 1;
        },
        false => 0,
    }
}

#[kani::proof]
fn match_loop_returns_count() {
    let n: u32 = kani::any();
    let run: bool = kani::any();
    assert_eq!(count_to(n, run), if run { n } else { 0 });
}

#[kani::proof]
#[kani::should_panic]
fn match_loop_detects_wrong_result() {
    let n: u32 = kani::any();
    kani::assume(n > 0);
    assert_eq!(count_to(n, true), 0);
}

#[rustfmt::skip]
fn countdown(mut n: u8, order: bool) -> u8 {
    match order {
        true =>
        #[kani::loop_decreases(n)]
        #[kani::loop_invariant(n <= 3)]
        #[kani::loop_modifies(&n)]
        while n > 0 { n -= 1; },
        false =>
        #[kani::loop_modifies(&n)]
        #[kani::loop_invariant(n <= 3)]
        #[kani::loop_decreases(n)]
        while n > 0 { n -= 1; },
    }
    n
}

#[kani::proof]
fn match_decreases_attribute_orders() {
    let n: u8 = kani::any();
    kani::assume(n <= 3);
    assert_eq!(countdown(n, kani::any()), 0);
}

#[rustfmt::skip]
#[kani::proof]
fn match_for_loop() {
    let mut count = 0u8;
    match true {
        true =>
        #[kani::loop_invariant(count == kani::index as u8)]
        #[kani::loop_modifies(&count)]
        for _ in 0..3 { count += 1; },
        false => (),
    }
    assert_eq!(count, 3);
}
