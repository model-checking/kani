// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --output-format old
fn foo() -> Option<i32> {
    None
}

fn main() {
    assert!(foo() == None);
    let x = foo();
    let y: Option<i32> = None;
    assert!(foo() == y);
}
