// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --output-format regular

// This test checks that kani injects a reachability check for debug_assert
// macro and that it reports ones that are unreachable.
// The check in this test is reachable and does not hold, so should be reported
// as FAILURE

fn check(x: i32) {
    if x < 0 {
        debug_assert!(x != -10);
    }
}

fn main() {
    check(-10);
}
