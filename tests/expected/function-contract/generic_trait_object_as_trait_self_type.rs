// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// A `dyn Trait` self type in a *qualified-trait* target (`<dyn TA as TA>::a`)
// must fail to resolve cleanly, like the plain `<dyn TA>::a` case. Since
// `resolve_ty` now builds `dyn TA`, this path would otherwise resolve to a
// vtable (`Virtual`) instance instead of erroring.

trait TA {
    fn a(&self) -> u8;
}

#[kani::proof_for_contract(<dyn TA as TA>::a)]
fn check_dyn_as_trait_self_type() {}
