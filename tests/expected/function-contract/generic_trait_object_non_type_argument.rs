// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// A non-type generic argument on the principal (`dyn Get<3>`) must fail to
// resolve cleanly.

use std::boxed::Box;

trait Get<T> {
    fn g(&self) -> T;
}

struct S<T>(T);
trait Probe {
    fn probe(&self) -> u32;
}

#[kani::proof_for_contract(<S<Box<dyn Get<3>>> as Probe>::probe)]
fn check_non_type_argument() {}
