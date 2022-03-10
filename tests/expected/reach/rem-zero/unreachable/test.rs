// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --output-format regular --no-default-checks

// This test checks that kani injects a reachability check for
// remainder-by-zero checks and that it reports ones that are unreachable.
// The check in this test is unreachable, so should be reported as UNREACHABLE

fn rem(x: u16, y: u16) -> u16 {
    if y != 0 { x % y } else { 0 }
}

fn main() {
    rem(5, 0);
}
