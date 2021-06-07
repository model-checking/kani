// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @flag --no-memory-splitting --unroll=4
// @expect verified

// cbmc-flags: --unwind 5 --unwinding-assertions

fn fac(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        _ => n * fac(n - 1),
    }
}

fn __nondet<T>() -> T {
    unimplemented!()
}

pub fn main() {
    let mut a = 1;
    let n = __nondet();
    if n < 5 {
        for i in 1..n + 1 as u64 {
            a *= i;
        }
        assert!(a == fac(n)); // a == 6!
    }
}
