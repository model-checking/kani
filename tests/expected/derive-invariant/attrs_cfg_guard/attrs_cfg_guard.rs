// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that the `#[safety_constraint(...)]` attribute helper is picked up
//! when it's used with `cfg_attr(kani, ...)]`.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
struct PositivePoint {
    #[cfg_attr(kani, safety_constraint(*x >= 0))]
    x: i32,
    #[cfg_attr(kani, safety_constraint(*y >= 0))]
    y: i32,
}

#[kani::proof]
fn check_safety_constraint_cfg() {
    let pos_point: PositivePoint = kani::any();
    assert!(pos_point.x >= 0);
    assert!(pos_point.y >= 0);
}
