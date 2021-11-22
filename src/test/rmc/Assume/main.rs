// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn main() {
    let i: i32 = rmc::nondet();
    rmc::assume(i < 10);
    assert!(i < 20);
}
