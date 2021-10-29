// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This benchmark tests the performance of the abstraction on the append()
// operation.

include!{"../benchmark-prelude.rs"}

fn operate_on_vec(times: usize) {
    let mut v: Vec<u32> = Vec::with_capacity(times);
    for i in 0..times {
        v.push(rmc::nondet());
    }
    let mut v2: Vec<u32> = Vec::with_capacity(times);
    for i in 0..times {
        v2.push(rmc::nondet());
    }
    v.append(&mut v2);
    assert!(v.len() == times * 2);
}

fn main() {
    operate_on_vec(TEST_SIZE);
}
