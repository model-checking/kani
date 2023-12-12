// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#![allow(unreachable_code, unused_variables)]

/// This only exists so I can fake a [`kani::Arbitrary`] instance for `*const
/// usize`.
struct ArbitraryPointer<P>(P);

impl kani::Arbitrary for ArbitraryPointer<*const usize> {
    fn any() -> Self {
        unreachable!()
    }
}

#[kani::ensures(true)]
fn return_pointer() -> ArbitraryPointer<*const usize> {
    unreachable!()
}

#[kani::proof_for_contract(return_pointer)]
fn return_ptr_harness() {
    return_pointer();
}
