// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --assertion-reach-checks --output-format regular

// This test checks that kani injects a reachability check for negation
// overflow checks and that it reports ones that are unreachable
// The negation overflow check in this test is reachable and doesn't hold, so
// should be reported as FAILURE

fn negate(x: i32) -> i32 {
    -x
}

fn main() {
    negate(std::i32::MIN);
}
