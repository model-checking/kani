// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// A `proof_for_contract` path written with NO generic arguments (`NoDefaultBare`, not
// `NoDefaultBare<..>`) keeps the uninstantiated type when a parameter has no default. The
// no-arguments path is treated as an empty argument list, but the substitution loop still
// returns the identity type at the first parameter it cannot fill (see
// tests/kani/FunctionContracts/generic_default_argument_fill.rs).

struct NoDefaultBare<T>(T);

trait Probe {
    fn probe(&self) -> u32;
}

impl Probe for NoDefaultBare<u8> {
    #[kani::ensures(|r| *r == 2)]
    fn probe(&self) -> u32 {
        2
    }
}

mod verify {
    use super::*;

    #[kani::proof_for_contract(<NoDefaultBare as Probe>::probe)]
    fn check_missing_required_bare() {
        let n = NoDefaultBare(1u8);
        let _ = n.probe();
    }
}
