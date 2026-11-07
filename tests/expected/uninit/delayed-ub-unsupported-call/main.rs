// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

//! The points-to analysis behind the delayed UB checks cannot follow calls through function
//! pointers or vtables. It used to crash on them; now they are reported as unsupported.

trait Calc {
    fn calc(&self, x: u32) -> u32;
}

struct Adder;

impl Calc for Adder {
    fn calc(&self, x: u32) -> u32 {
        x.wrapping_add(1)
    }
}

#[kani::proof]
fn check_dyn_call() {
    let adder = Adder;
    let dyn_ref: &dyn Calc = &adder;
    let result = dyn_ref.calc(kani::any());
    assert!(result < 200 || result >= 200);
}

fn double(x: u32) -> u32 {
    x.wrapping_mul(2)
}

#[kani::proof]
fn check_fn_ptr_call() {
    let f: fn(u32) -> u32 = double;
    let x: u32 = kani::any();
    assert_eq!(f(x), x.wrapping_mul(2));
}
