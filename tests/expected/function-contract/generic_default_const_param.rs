// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// A `proof_for_contract` path that omits a generic parameter with a declared default keeps
// the uninstantiated type when that parameter is a CONST parameter: only defaulted TYPE
// parameters are filled (see tests/kani/FunctionContracts/generic_default_argument_fill.rs).
// `WithConst<u8>` omits `N` (default 4); the const default is not filled, so the target
// keeps the uninstantiated type and fails to resolve.

struct WithConst<T, const N: usize = 4>([T; N]);

trait Probe {
    fn probe(&self) -> u32;
}

impl Probe for WithConst<u8> {
    #[kani::ensures(|r| *r == 0)]
    fn probe(&self) -> u32 {
        0
    }
}

mod verify {
    use super::*;

    #[kani::proof_for_contract(<WithConst<u8> as Probe>::probe)]
    fn check_const_default_not_filled() {
        let w = WithConst([0u8; 4]);
        let _ = w.probe();
    }
}
