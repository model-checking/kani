// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// Two principal (non-auto) traits (`dyn TA + TB`) must fail to resolve cleanly.

use std::boxed::Box;

trait TA {
    fn a(&self) -> u8;
}
trait TB {
    fn b(&self) -> u8;
}

struct S<T>(T);
trait Probe {
    fn probe(&self) -> u32;
}

#[kani::proof_for_contract(<S<Box<dyn TA + TB>> as Probe>::probe)]
fn check_two_principals() {}
