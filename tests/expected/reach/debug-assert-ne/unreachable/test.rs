// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that kani injects a reachability check for debug_assert_ne
// macro and that it reports ones that are unreachable.
// The check in this test is unreachable, so should be reported as UNREACHABLE

fn check(x: i32) {
    if x > 5 {
        debug_assert_ne!(x, 17);
    }
}

fn main() {
    check(1);
}
