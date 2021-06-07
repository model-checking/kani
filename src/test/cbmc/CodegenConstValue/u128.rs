// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Avoid RUST constant propagating this away
fn assert_bigger(a: u128, b: u128) {
    assert!(a > b);
}

fn main() {
    assert_bigger(u128::MAX, 12);
}
