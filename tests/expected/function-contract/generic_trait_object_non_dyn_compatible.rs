// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// A non-dyn-compatible principal (`dyn NotObj`, a generic method) must fail to
// resolve cleanly.

use std::boxed::Box;

trait NotObj {
    fn method<U>(&self, u: U);
}

struct S<T>(T);
trait Probe {
    fn probe(&self) -> u32;
}

#[kani::proof_for_contract(<S<Box<dyn NotObj>> as Probe>::probe)]
fn check_non_dyn_compatible() {}
