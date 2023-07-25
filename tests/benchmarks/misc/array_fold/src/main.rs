// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks the performance of fold, which uses iterators under the
//! hood.
//! The test is from https://github.com/model-checking/kani/issues/1823.
//! Pre CBMC 5.72.0, it took ~36 seconds and consumed ~3.6 GB of memory.
//! With CBMC 5.72.0, it takes ~11 seconds and consumes ~255 MB of memory.

pub fn array_sum_fold(x: [usize; 100]) -> usize {
    x.iter().fold(0, |accumulator, current| accumulator + current)
}

pub fn array_sum_for(x: [usize; 100]) -> usize {
    let mut accumulator: usize = 0;
    for i in 0..100 {
        accumulator = x[i] + accumulator
    }
    accumulator
}

#[kani::proof]
fn array_sum_fold_proof() {
    let x: [usize; 100] = kani::any();
    array_sum_fold(x);
}

#[kani::proof]
fn array_sum_for_proof() {
    let x: [usize; 100] = kani::any();
    array_sum_for(x);
}

fn main() {}
