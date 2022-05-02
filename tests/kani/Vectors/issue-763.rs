// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Failing example from https://github.com/model-checking/kani/issues/763
#[kani::proof]
fn main() {
    let x = Vec::<i32>::new();
    for i in x {}
}
