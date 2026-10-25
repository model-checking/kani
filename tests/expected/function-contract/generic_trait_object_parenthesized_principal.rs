// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// Parenthesized trait arguments on the principal (`dyn TA(u8)`) are not the
// angle-bracketed form the arm handles. They must be rejected, not silently
// dropped to `dyn TA` — which would match the impl below.

use std::boxed::Box;

trait TA {
    fn a(&self) -> u8;
}

struct S<T>(T);
trait Probe {
    fn probe(&self) -> u32;
}

// The impl a dropped-args resolution would wrongly match.
impl Probe for S<Box<dyn TA>> {
    #[kani::ensures(|r| *r == 1)]
    fn probe(&self) -> u32 {
        1
    }
}

#[kani::proof_for_contract(<S<Box<dyn TA(u8)>> as Probe>::probe)]
fn check_parenthesized_principal() {}
