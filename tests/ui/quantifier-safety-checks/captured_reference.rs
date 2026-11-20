// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers -Z valid-value-checks --solver cadical

//! A predicate that reads a `bool` through a captured reference: with `-Z valid-value-checks`, the
//! read is checked in case its bytes were written without a type check, and a quantifier
//! predicate can't contain checks, so Kani rejects it. Values captured by value are not checked.
//! Tracked in <https://github.com/model-checking/kani/issues/5193>.

#[kani::proof]
fn reads_through_captured_reference() {
    let flags: [bool; 2] = kani::any();
    let flags = &flags;
    assert!(kani::forall!(|i in (0, 2)| flags[i] || !flags[i]));
}
