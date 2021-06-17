// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
static X: i32 = 12;

fn foo() -> i32 {
    X
}

fn main() {
    assert!(10 == foo());
    assert!(12 == foo());
}
