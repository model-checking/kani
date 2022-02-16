// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// This test checks that Kani injects a reachability check for the assert_eq
// macro. The test has an unreachable assert_eq statement which should be
// reported as UNREACHABLE

// kani-flags: --assertion-reach-checks --output-format regular --no-default-checks

fn main() {
    let x: i32 = kani::any();
    let y = if x > 10 { 15 } else { 33 };
    if y > 50 {
        assert_eq!(y, 55);
    }
}
