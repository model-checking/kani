// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn check_contradictory_assume() {
    let x: u8 = kani::any();
    kani::assume(x > 10 && x < 5);
    assert!(x < 5);
    assert!(x > 10);
}

#[kani::proof]
fn check_real_failure() {
    let x: u8 = kani::any();
    kani::assume(x < 5);
    assert!(x > 10);
}

#[kani::proof]
fn check_real_success() {
    let x: u8 = kani::any();
    kani::assume(x < 100);
    assert!(x < 200);
}

#[kani::proof]
fn check_checkless_success() {}

#[kani::proof]
#[kani::should_panic]
fn check_should_panic_success() {
    let x: u8 = kani::any();
    assert!(x != x, "always panics");
}

#[kani::proof]
#[kani::should_panic]
fn check_should_panic_failing_vacuous() {
    let x: u8 = kani::any();
    kani::assume(x > 10 && x < 5);
    assert!(x < 5);
    assert!(x > 10);
}
