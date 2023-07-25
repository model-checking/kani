// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks the performance of 2d-vectors
//! The test is from https://github.com/model-checking/kani/issues/1226.
//! Pre CBMC 5.72.0, it ran out of memory.
//! With CBMC 5.72.0, it takes ~2 seconds and consumes a few hundred MB of memory.

#[kani::proof]
#[kani::unwind(5)]
#[kani::solver(minisat)]
fn main() {
    let v1: Vec<Vec<i32>> = vec![vec![1], vec![]];

    let v2: Vec<i32> = v1.into_iter().flatten().collect();
    assert_eq!(v2, vec![1]);
}
