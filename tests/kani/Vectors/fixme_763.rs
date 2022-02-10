// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Failing example from https://github.com/model-checking/kani/issues/763
fn main() {
    let x = Vec::<i32>::new();
    for i in x {}
}
