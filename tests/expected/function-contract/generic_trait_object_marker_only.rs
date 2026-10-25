// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// A marker-only object with no principal trait (`dyn Send`) must be declined,
// not matched to the valid `impl Probe for S<Box<dyn Send>>` below.

use std::boxed::Box;
use std::marker::Send;

struct S<T>(T);
trait Probe {
    fn probe(&self) -> u32;
}

impl Probe for S<Box<dyn Send>> {
    #[kani::ensures(|r| *r == 9)]
    fn probe(&self) -> u32 {
        9
    }
}

#[kani::proof_for_contract(<S<Box<dyn Send>> as Probe>::probe)]
fn check_marker_only() {
    let s = S(Box::new(0u8) as Box<dyn Send>);
    let _ = s.probe();
}
