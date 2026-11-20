// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// A fn-pointer argument with a non-Rust ABI is not built by the `Type::BareFn` arm;
// the path keeps the uninstantiated type and fails to resolve, as before (see
// tests/kani/FunctionContracts/generic_fn_pointer_argument.rs for the supported cases).

struct Wrap<T>(T);

trait Probe {
    fn probe(&self) -> u32;
}

impl Probe for Wrap<extern "C" fn(u8) -> u8> {
    #[kani::ensures(|r| *r == 1)]
    fn probe(&self) -> u32 {
        1
    }
}

mod verify {
    use super::*;

    #[kani::proof_for_contract(<Wrap<extern "C" fn(u8) -> u8> as Probe>::probe)]
    fn check_c_abi_fn_pointer() {
        extern "C" fn id(x: u8) -> u8 {
            x
        }
        let w = Wrap(id as extern "C" fn(u8) -> u8);
        let _ = w.probe();
    }
}
