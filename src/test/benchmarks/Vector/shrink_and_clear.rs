// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This benchmark checks the performance of the abstraction for its shrink and
// truncate operations.
include!{"../benchmark-prelude.rs"}

fn operate_on_vec(times: usize) {
    // Create vector with known capacity
    let mut v: Vec<u32> = Vec::with_capacity(times);
    for i in 0..times {
        v.push(kani::any());
    }
    assert!(v.len() == times);
    // Here, Vecs with grow() internally
    let i: usize = kani::any();
    kani::assume(i >= 0 && i < v.len());
    let saved = v[i];
    // Completely shrink the array to remove additional allocations
    v.shrink_to_fit();
    assert!(v[i] == saved);
    // Push some new elements to grow() again
    for i in 0..times {
        v.push(kani::any());
    }
    // Drop all elements in the Vec
    v.clear();
    // Add some more new elements
    for i in 0..times {
        v.push(kani::any());
    }
    // Assert!
    let sentinel = kani::any();
    v.push(sentinel);
    assert!(v.pop() == Some(sentinel));
}

fn main() {
    operate_on_vec(TEST_SIZE);
}
