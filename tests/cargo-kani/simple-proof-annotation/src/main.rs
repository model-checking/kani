// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn main() {
    assert!(1 == 2);
}

// NOTE: Currently the below is not detected or run by this test!

// The expected file presently looks for "1 == 2" above.
// But eventually this test may start to fail as we might stop regarding 'main'
// as a valid proof harness, since it isn't annotated as such.
// This test should be updated if we go that route.

#[kani::proof]
fn harness() {
    assert!(3 == 4);
}
