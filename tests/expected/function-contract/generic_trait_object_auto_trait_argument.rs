// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// Generic arguments on an auto trait (`dyn TA + Send<u8>`) are malformed (auto
// traits are parameterless). They must be rejected, not silently dropped to
// `dyn TA + Send` — which would match the impl below.

use std::boxed::Box;
use std::marker::Send;

trait TA {
    fn a(&self) -> u8;
}

struct S<T>(T);
trait Probe {
    fn probe(&self) -> u32;
}

// The impl a dropped-args resolution would wrongly match.
impl Probe for S<Box<dyn TA + Send>> {
    #[kani::ensures(|r| *r == 3)]
    fn probe(&self) -> u32 {
        3
    }
}

#[kani::proof_for_contract(<S<Box<dyn TA + Send<u8>>> as Probe>::probe)]
fn check_auto_trait_argument() {}
