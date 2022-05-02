// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
#[kani::unwind(6)]
fn main() {
    let name: &str = "hello";
    assert!(name == "hello");
}
