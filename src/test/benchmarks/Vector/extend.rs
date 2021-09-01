// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This benchmark tests the performance of the abstraction on the extend()
// operation.

include!{"../benchmark-prelude.rs"}

fn operate_on_vec(times: usize) {
    let mut v = Vec::with_capacity(times);
    v.extend(1..times);
}

fn main() {
    operate_on_vec(TEST_SIZE);
}
