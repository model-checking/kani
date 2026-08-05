// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts
//! Test that implementation contracts take precedence over default trait function contracts

trait MyTrait {
    #[kani::ensures(|result| *result == 42)]
    fn get_value(&self) -> u32 {
        42
    }
}

struct MyStruct;

impl MyTrait for MyStruct {
    #[kani::ensures(|result| *result == 100)]
    fn get_value(&self) -> u32 {
        100
    }
}

#[kani::proof_for_contract(<MyStruct as MyTrait>::get_value)]
fn test_impl_contract_used() {
    let s = MyStruct;
    s.get_value();
}
