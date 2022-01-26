// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --unwind 3
// Check that the Invariant implementation for array respect the underlying types invariant.

extern crate kani;

use kani::Invariant;

fn main() {
    let arr: [char; 2] = kani::any();
    assert!(arr[0].is_valid());
    assert!(arr[1].is_valid());
}
