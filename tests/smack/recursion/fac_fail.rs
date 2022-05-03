// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @flag --unroll=10
// @expect error
// kani-verify-fail

fn fac(n: u64, acc: u64) -> u64 {
    match n {
        0 => acc,
        _ => fac(n - 1, acc * n),
    }
}

pub fn main() {
    let x = fac(5, 1);
    assert!(x != 120);
}
