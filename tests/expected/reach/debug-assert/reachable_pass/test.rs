// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --assertion-reach-checks --output-format regular

// This test checks that kani injects a reachability check for debug_assert
// macro and that it reports ones that are unreachable.
// The check in this test is reachable, so should be reported as SUCCESS

fn check(x: i32) {
    if x < 0 {
        debug_assert!(x != -10);
    }
}

fn main() {
    check(-9);
}
