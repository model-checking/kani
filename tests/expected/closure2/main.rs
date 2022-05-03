// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    let x = 2;
    let f = |y| x + y;
    let z = f(100);
    let g = |y| z + f(y);
    assert!(z == 102);
    assert!(g(z) == 206);
}
