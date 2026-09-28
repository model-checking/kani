// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// A `proof_for_contract` path that omits a generic parameter WITHOUT a declared default
// (`NoDefault<u8>` for `NoDefault<T, U>`) keeps the uninstantiated type and fails to
// resolve. Only parameters with declared defaults are filled (see
// tests/kani/FunctionContracts/generic_default_argument_fill.rs).

struct NoDefault<T, U>(T, U);

trait Probe {
    fn probe(&self) -> u32;
}

impl Probe for NoDefault<u8, u16> {
    #[kani::ensures(|r| *r == 1)]
    fn probe(&self) -> u32 {
        1
    }
}

mod verify {
    use super::*;

    #[kani::proof_for_contract(<NoDefault<u8> as Probe>::probe)]
    fn check_missing_required_param() {
        let n = NoDefault(1u8, 2u16);
        let _ = n.probe();
    }
}
