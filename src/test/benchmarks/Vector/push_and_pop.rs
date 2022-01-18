// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This benchmark tests the performance of the abstraction on push and pop operations.
// This is also used to test the noback vec abstraction

include!{"../benchmark-prelude.rs"}

fn operate_on_vec(times: usize) {
    let mut v: Vec<u32> = Vec::new();
    for i in 0..times {
        v.push(kani::any());
    }
    assert!(v.len() == times);
    v.push(1);
    assert!(v.pop() == Some(1));
}

fn operate_on_vec_len(times: usize) {
    let mut v: Vec<u32> = Vec::new();
    for i in 0..times {
        v.push(kani::any());
    }
    assert!(v.len() == times);
    v.push(1);
    assert!(v.pop() == Some(1));
}

fn main() {
    operate_on_vec_len(5);
}
