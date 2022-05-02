// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that kani injects a reachability check for negation
// overflow checks and that it reports ones that are unreachable
// The negation overflow check in this test is unreachable, so should be
// reported as UNREACHABLE

fn negate(x: i32) -> i32 {
    if x != std::i32::MIN { -x } else { std::i32::MAX }
}

#[kani::proof]
fn main() {
    negate(std::i32::MIN);
}
