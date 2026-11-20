// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// A `dyn Trait` *self* type in a contract target must fail to resolve cleanly
// (no ICE). `resolve_ty` now builds `dyn TA`, so `<dyn TA>::a` reaches
// `resolve_in_user_type`, which must decline it rather than hit `unreachable!`.
// (No contract on `a` is needed: target resolution fails before any
// contract-existence check.)

trait TA {
    fn a(&self) -> u8;
}

#[kani::proof_for_contract(<dyn TA>::a)]
fn check_dyn_self_type() {}
