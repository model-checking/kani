// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This benchmark checks the performance of the abstraction for its insert and
// remove operations. They are particularly interesting as they perform memory 
// move operations to insert element.
include!{"../benchmark-prelude.rs"}

fn operate_on_vec(times: usize) {
    let mut v: Vec<u32> = Vec::with_capacity(times);
    for i in 0..times {
        v.push(unsafe { rmc::nondet() });
    }
    let sentinel = unsafe { rmc::nondet() };
    v.push(sentinel);
    v.insert(v.len()/2, unsafe { rmc::nondet() });
    assert!(v.pop() == Some(sentinel));
}

fn main() {
    operate_on_vec(TEST_SIZE);
}
