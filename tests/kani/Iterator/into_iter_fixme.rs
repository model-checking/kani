// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// compile-flags: --edition 2018
// kani-flags: --unwind 4 --cbmc-args --object-bits 9
//
// This reproduces the issue seen in "Failures when iterating over results".
// See https://github.com/model-checking/kani/issues/556 for more information.
#[kani::proof]
pub fn main() {
    let numbers = vec![1, 10, -1];
    let positives: Vec<_> = numbers.into_iter().filter(|&n| n > 0).collect();
    assert_eq!(positives.len(), 2);
}
