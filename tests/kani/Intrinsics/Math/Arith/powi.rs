// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn verify_powi32() {
    let x: f32 = kani::any();
    kani::assume(x.is_normal());
    kani::assume(x >= 1e-19 || x <= -1e-19);
    kani::assume(x <= 1.84e19 && x >= -1.84e19);
    let x2 = x.powi(2);
    assert!(x2 >= 0.0);
}

#[kani::proof]
fn verify_powi64() {
    let x: f64 = kani::any();
    kani::assume(x.is_normal());
    kani::assume(x >= 1e-153 || x <= -1e-153);
    kani::assume(x <= 1.34e154 && x >= -1.34e154);
    let x2 = x.powi(2);
    assert!(x2 >= 0.0);
}
