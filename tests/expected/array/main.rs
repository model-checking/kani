// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// cbmc-flags: --bounds-check
fn foo(x: [i32; 5]) -> [i32; 2] {
    [x[0], x[1]]
}

/// Generate an out-of-bound index with the given length.
/// We use a function so the constant propagation
fn oob_index(len: usize) -> usize {
    len
}

#[kani::proof]
fn main() {
    let x = [1, 2, 3, 4, 5];
    let y = foo(x);
    let z = oob_index(y.len());
    assert!(y[0] == 1);
    assert!(y[1] == 2);
    assert!(y[z] == 3);
}
