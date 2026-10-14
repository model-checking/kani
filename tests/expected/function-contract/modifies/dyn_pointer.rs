// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! A `modifies` clause on a pointer to a trait object used to crash the compiler. It covers the
//! object behind the pointer, whose size the vtable records.

trait Demo {
    fn bump(&mut self);
}

struct Counter {
    n: u32,
    m: u64,
}

impl Demo for Counter {
    fn bump(&mut self) {
        self.n = self.n.wrapping_add(1);
        self.m = 7;
    }
}

#[kani::modifies(ptr)]
fn touch_dyn(ptr: *mut dyn Demo) {
    unsafe { (*ptr).bump() }
}

#[kani::proof_for_contract(touch_dyn)]
fn check_touch_dyn() {
    let mut value = Counter { n: kani::any(), m: 0 };
    let ptr: *mut dyn Demo = &mut value;
    touch_dyn(ptr);
}

static mut OTHER: u32 = 0;

#[kani::modifies(ptr)]
fn touch_other(ptr: *mut dyn Demo) {
    let _ = ptr;
    unsafe { OTHER = 1 }
}

#[kani::proof_for_contract(touch_other)]
fn check_touch_other() {
    let mut value = Counter { n: 0, m: 0 };
    let ptr: *mut dyn Demo = &mut value;
    touch_other(ptr);
}
