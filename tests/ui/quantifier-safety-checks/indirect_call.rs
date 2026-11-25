// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers -Z valid-value-checks -Z uninit-checks --solver cadical

#[inline(never)]
fn first(i: usize) -> bool {
    i < 2
}

#[inline(never)]
fn second(i: usize) -> bool {
    i == 0
}

#[kani::proof]
fn indirect_predicate() {
    let predicate: fn(usize) -> bool = if kani::any() { first } else { second };
    // Captured by `move`: a capture read through a reference would be rejected first, as a
    // safety check in the predicate (see `captured_reference.rs`).
    assert!(kani::internal::kani_forall(0, 2, move |i: usize| predicate(i)));
}
