// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn test() {
    let x = 4;
    let y = foo::foo(x);
    assert!(y == x);
}
