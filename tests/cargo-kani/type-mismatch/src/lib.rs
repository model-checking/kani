// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[kani::proof]
fn main() {
    let arr = [1, 2, 3];
    let r: core::ops::Range<usize> = uses_core::foo(&arr[..2]);
    let i = uses_std::bar(r);
    assert!(i.start == 10);
}
