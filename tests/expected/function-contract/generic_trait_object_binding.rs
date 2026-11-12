// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// A trait-object argument with an associated-type binding (`dyn Bind<Item = u8>`)
// is not built by the `Type::TraitObject` arm; the path keeps the uninstantiated
// type and fails to resolve (see
// tests/kani/FunctionContracts/generic_trait_object_argument.rs for supported cases).

use std::boxed::Box;

trait Bind {
    type Item;
    fn item(&self) -> Self::Item;
}

struct S<T>(T);
trait Probe {
    fn probe(&self) -> u32;
}

impl Probe for S<Box<dyn Bind<Item = u8>>> {
    #[kani::ensures(|r| *r == 1)]
    fn probe(&self) -> u32 {
        1
    }
}

struct I;
impl Bind for I {
    type Item = u8;
    fn item(&self) -> u8 {
        0
    }
}

#[kani::proof_for_contract(<S<Box<dyn Bind<Item = u8>>> as Probe>::probe)]
fn check_binding() {
    let s = S(Box::new(I) as Box<dyn Bind<Item = u8>>);
    let _ = s.probe();
}
