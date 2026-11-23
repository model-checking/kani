// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn b_light_in_extra_rs() {
    let y: u32 = kani::any();
    kani::assume(y < 4);
    assert!(y < 4);
}

#[kani::proof]
fn c_light_in_extra_rs() {
    let z: u32 = kani::any();
    kani::assume(z < 4);
    assert!(z < 4);
}
