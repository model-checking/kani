
// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#[kani::ensures(result.is_some())]
fn or_default<T: Default + kani::Arbitrary>(opt: Option<T>) -> Option<T> {
    opt.or(Some(T::default()))
}

#[kani::proof_for_contract(or_default)]
fn harness() {
    let input: Option<i32> = kani::any();
    or_default(input);
}