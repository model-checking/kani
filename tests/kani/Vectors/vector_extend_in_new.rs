// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we can handle set len on drop for an implicit extend with
// the vec![i; j] constructor.

// Temporarily disabling assertion reachability checks because they trigger a
// CBMC crash
// https://github.com/diffblue/cbmc/issues/6691
// https://github.com/model-checking/kani/issues/861
// kani-flags: --no-assertion-reach-checks

// There is an implicit loop, so we need an explicit unwind
// cbmc-flags: --unwind 3 --unwinding-assertions

fn main() {
    let a: Vec<Vec<i32>> = vec![vec![0; 2]; 1];
    assert!(a.len() == 1);
}
