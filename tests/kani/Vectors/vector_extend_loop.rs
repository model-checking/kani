// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --default-unwind 3

#[kani::proof]
#[kani::unwind(3)]
fn main() {
    let mut v: Vec<u32> = Vec::new();
    for (start, len) in vec![(0, 1), (1, 2)] {
        v.extend(start..=(start + len - 1));
    }
}
