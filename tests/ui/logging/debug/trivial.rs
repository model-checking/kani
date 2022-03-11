// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --function harness --debug
// This test is to make sure we are correctly printing debug messages in the compiler.
//
// We don't rely in a specific debug message since they can change at any point. We do however
// expect at least one debug message to be printed.

#[kani::proof]
fn harness() {
    let v = vec![1, 2];
    assert_ne!(v[0], v[1]); 
}
