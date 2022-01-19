// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This benchmark checks the performance of the abstraction for its insert and
// remove operations. They are particularly interesting as they perform memory 
// move operations to insert element.
include!{"../benchmark-prelude.rs"}

fn operate_on_vec(times: usize) {
    let mut v: Vec<u32> = Vec::with_capacity(times);
    for i in 0..times {
        v.push(kani::any());
    }
    let sentinel = kani::any();
    v.push(sentinel);
    // We remove elements to perform more memmoves. These are done to fill up
    // "holes" created due to elements removed from the middle of the Vec.
    for i in 0..times/2 {
        v.remove(v.len()/2);
    }
    assert!(v.pop() == Some(sentinel));
    assert!(v.len() == times/2 || v.len() == (times/2 + 1));
}

fn main() {
    operate_on_vec(TEST_SIZE);
}
