// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags:

#[kani::proof]
#[kani::stub(std::vec::Vec::is_empty, kani::stubs::vec::is_empty)]
fn new_test() {
    let v: Vec<i32> = Vec::new();
    assert!(v.is_empty());
}
