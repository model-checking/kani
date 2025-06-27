// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test is to check that we have addressed the performance issue called out in
//! https://github.com/model-checking/kani/issues/2363.

#[kani::proof]
#[kani::unwind(7)]
#[kani::solver(cadical)]
fn main() {
    let s = "Mary had a little lamb";
    let v: Vec<&str> = s.split(' ').collect();
    assert_eq!(v, ["Mary", "had", "a", "little", "lamb"]);
}
