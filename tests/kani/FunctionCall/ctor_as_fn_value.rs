// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that ADT constructors defined in other crates can be used as function
//! values. As of nightly-2026-02-06, rustc no longer encodes optimized MIR for
//! ADT constructors, so their bodies are unavailable through
//! `rustc_public::Instance::body()` for constructors imported from other crates
//! and Kani must reconstruct them (see `BodyTransformation::body`). Passing
//! `Some` or `std::num::Wrapping` to `Iterator::map` exercises exactly that
//! path; a local constructor is included for contrast.

struct Local(u8);

#[kani::proof]
fn check_cross_crate_ctor_as_fn_value() {
    let x: u8 = kani::any();
    // `Some` is `Option`'s constructor, defined in `core`.
    let v = std::iter::once(x).map(Some).next().unwrap();
    assert_eq!(v, Some(x));
    // A tuple-struct constructor defined in `core` (re-exported by `std`).
    let w = std::iter::once(x).map(std::num::Wrapping).next().unwrap();
    assert_eq!(w.0, x);
}

#[kani::proof]
fn check_local_ctor_as_fn_value() {
    let x: u8 = kani::any();
    let l = std::iter::once(x).map(Local).next().unwrap();
    assert_eq!(l.0, x);
}
