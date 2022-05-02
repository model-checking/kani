// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we can pass a dyn FnOnce pointer to a stand alone
// function definition.

fn takes_dyn_fun(fun: Box<dyn FnOnce() -> u32>) -> u32 {
    fun()
}

pub fn unit_to_u32() -> u32 {
    5
}

#[kani::proof]
fn main() {
    assert!(takes_dyn_fun(Box::new(&unit_to_u32)) == 5)
}
