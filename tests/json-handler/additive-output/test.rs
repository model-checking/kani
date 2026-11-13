// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn add_numbers(a: u32, b: u32) -> u32 {
    a + b
}

#[kani::proof]
fn verify_add_numbers() {
    let x: u32 = kani::any();
    let y: u32 = kani::any();
    kani::assume(x < 100);
    kani::assume(y < 100);
    let result = add_numbers(x, y);
    assert!(result >= x);
    assert!(result >= y);
}

#[kani::proof]
fn verify_add_numbers_fails() {
    let x: u32 = kani::any();
    let y: u32 = kani::any();
    kani::assume(x < 100);
    kani::assume(y < 100);
    let result = add_numbers(x, y);
    assert!(result > x + y, "deliberately false for the additive-output failing-harness case");
}
