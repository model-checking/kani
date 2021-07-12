// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we can pass a dyn FnOnce pointer to a stand alone
// function definition. Expected to fail because we are comparing
// to an incorrect value.

include!("../../rmc-prelude.rs");

fn takes_dyn_fun(fun: Box<dyn FnOnce() -> u32>) -> u32 {
    fun()
}

pub fn unit_to_u32() -> u32 {
    5
}

fn main() {
    __VERIFIER_expect_fail(takes_dyn_fun(Box::new(&unit_to_u32)) == 3, "Wrong u32")
}
