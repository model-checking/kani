// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    let slice = &[1, 2, 3][..];
    if let [head, tail @ ..] = slice {
        assert!(head == &slice[0]);
        assert!(tail == &slice[1..]);
    } else {
        unreachable!();
    }
}
