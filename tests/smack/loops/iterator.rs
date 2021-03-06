// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn fac(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        _ => n * fac(n - 1),
    }
}

#[kani::proof]
#[kani::unwind(5)]
fn main() {
    let mut a = 1;
    let n = kani::any();
    if n < 5 {
        for i in 1..n + 1 as u64 {
            a *= i;
        }
        assert!(a == fac(n)); // a == 6!
    }
}
