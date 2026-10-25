// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// A trait-object argument over a trait with an unspecified associated type
// (`dyn Bind`, no `Item = ..`) is not built by the `Type::TraitObject` arm; it
// degrades to a clean resolution error (no ICE), as the written-binding case does
// (see generic_trait_object_binding.rs).

use std::boxed::Box;

trait Bind {
    type Item;
    fn item(&self) -> Self::Item;
}

struct S<T>(T);
trait Probe {
    fn probe(&self) -> u32;
}

#[kani::proof_for_contract(<S<Box<dyn Bind>> as Probe>::probe)]
fn check_unspecified_assoc() {}
