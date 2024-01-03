// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts
#![allow(unreachable_code, unused_variables)]

extern crate kani;

static mut B: bool = false;

/// This only exists so I can fake a [`kani::Arbitrary`] instance for `&mut
/// bool`.
struct ArbitraryPointer<P>(P);

impl<'a> kani::Arbitrary for ArbitraryPointer<&'a mut bool> {
    fn any() -> Self {
        ArbitraryPointer(unsafe { &mut B as &'a mut bool })
    }
}

#[kani::ensures(true)]
fn allowed_mut_return_ref<'a>() -> ArbitraryPointer<&'a mut bool> {
    ArbitraryPointer(unsafe { &mut B as &'a mut bool })
}

#[kani::proof_for_contract(allowed_mut_return_ref)]
fn allowed_mut_return_ref_harness() {
    let _ = Box::new(());
    allowed_mut_return_ref();
}
