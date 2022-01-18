// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This benchmark tests the performance of the abstraction for push() operations
// when the capacity is known

include!{"../benchmark-prelude.rs"}

fn operate_on_vec(times: usize) {
    let mut v: Vec<u32> = Vec::with_capacity(times);
    for i in 0..times {
        v.push(kani::any());
    }
}

fn main() {
    operate_on_vec(100);
}
