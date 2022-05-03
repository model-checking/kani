// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[kani::proof]
fn main() {
    let a: bool = kani::any();
    let b: bool = kani::any();
    let c = a ^ b;
    assert!((a == b && !c) || (a != b && c));
}
