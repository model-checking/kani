// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --output-format regular

// This test checks that kani injects a reachability check for negation
// overflow checks and that it reports ones that are unreachable
// The negation overflow check in this test is reachable, so should be
// reported as SUCCESS

fn negate(x: i32) -> i32 {
    if x != std::i32::MIN { -x } else { std::i32::MAX }
}

fn main() {
    negate(7532);
}
