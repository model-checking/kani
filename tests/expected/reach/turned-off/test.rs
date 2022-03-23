// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that Kani reports SUCCESS for an unreachable check when the
// option to turn off assertion reachability checks is specified

// kani-flags: --no-assertion-reach-checks

#[kani::proof]
fn main() {
    let x = if kani::any() { 5 } else { 9 };
    if x > 10 {
        assert!(x != 11);
    }
}
