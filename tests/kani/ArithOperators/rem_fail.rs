// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Check that remainder triggers overflow checks.
// Covers the case where `x == T::MIN && y == -1`.

#[kani::proof]
fn main() {
    let a: i8 = i8::MIN;
    let b: i8 = kani::any();
    kani::assume(b == -1);
    let _ = a % b;
}
