// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! Check that no pointer-validity checks are generated for the closure-capture
//! loads inside contract-clause closures: evaluating `x > 0 && y > 0` below
//! loads `x` and `y` through references the contract instrumentation itself
//! created from live locals, which cannot fail. The expected file pins the
//! total number of checks, which would jump by a handful of six-check
//! `pointer_dereference` groups per capture and closure if these vacuous
//! checks came back.//!
//! The total is plumbing-sensitive: nightly-2026-06-01 emits two fewer checks here (contract-clause
//! bookkeeping, unrelated to captures). What the count guards against is the number *jumping* by
//! six-check `pointer_dereference` groups, which it still does.

#[kani::requires(x > 0 && y > 0)]
fn sum(x: i32, y: i32) -> i32 {
    x.wrapping_add(y)
}

#[kani::proof]
fn harness() {
    let _ = sum(1, 2);
}
