// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn fib(x: u64) -> u64 {
    match x {
        0 => 0,
        1 => 1,
        _ => fib(x - 1) + fib(x - 2),
    }
}

#[kani::proof]
fn main() {
    let x = fib(6);
    assert!(x == 8);
}
