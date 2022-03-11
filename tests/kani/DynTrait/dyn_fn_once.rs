// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we can pass a dyn FnOnce pointer to a stand alone
// function definition.

// Temporarily disabling assertion reachability checks because they trigger a
// CBMC crash
// https://github.com/diffblue/cbmc/issues/6691
// https://github.com/model-checking/kani/issues/861
// kani-flags: --no-assertion-reach-checks

fn takes_dyn_fun(fun: Box<dyn FnOnce() -> u32>) -> u32 {
    fun()
}

pub fn unit_to_u32() -> u32 {
    5
}

fn main() {
    assert!(takes_dyn_fun(Box::new(&unit_to_u32)) == 5)
}
