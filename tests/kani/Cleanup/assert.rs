// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// This test ensures that kani follows the correct CFG for assertion failures.
// - Statements that succeeds an assertion failure should be unreachable.
// - Cleanup statements should still be executed though.
// - Note that failures while unwinding actually crashes the process. So drop may only be called
//   once.
// File: cleanup.rs
// kani-verify-fail

#[derive(PartialEq, Eq)]
struct S {
    a: u8,
    b: u16,
}

impl Drop for S {
    fn drop(&mut self) {
        assert!(false, "A1: This should still fail during cleanup");
    }
}

#[kani::proof]
fn main() {
    let lhs = S { a: 42, b: 42 };
    let rhs = S { a: 0, b: 0 };
    assert!(lhs == rhs, "A2: A very false statement. Always fail.");
    assert!(false, "A3: Unreachable assert. Code always panic before this line.");
}
