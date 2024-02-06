// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn verify_powi32() {
    let x: f32 = kani::any();
    kani::assume(x.is_normal());
    let x2 = x.powi(2);
    assert!(x2 >= 0);
}

#[kani::proof]
fn verify_powi64() {
    let x: f64 = kani::any();
    kani::assume(x.is_normal());
    let x2 = x.powi(2);
    assert!(x2 >= 0);
}
