// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we can pass a dyn FnMut pointer to a stand alone
// function definition.

// Temporarily disabling assertion reachability checks because they trigger a
// CBMC crash
// https://github.com/diffblue/cbmc/issues/6691
// https://github.com/model-checking/kani/issues/861
// kani-flags: --no-assertion-reach-checks

fn takes_dyn_fun(mut fun: Box<dyn FnMut(&mut i32)>, x_ptr: &mut i32) {
    fun(x_ptr)
}

pub fn mut_i32_ptr(x: &mut i32) {
    *x = *x + 1;
}

fn main() {
    let mut x = 1;
    takes_dyn_fun(Box::new(&mut_i32_ptr), &mut x);
    assert!(x == 2);

    takes_dyn_fun(Box::new(&mut_i32_ptr), &mut x);
    assert!(x == 3);
}
