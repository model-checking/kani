// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::ensures(true)]
fn always() {}

#[kani::proof_for_contract(always)]
fn always_harness() {
    let _ = Box::new(());
    always();
}
