// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --function foo --assertion-reach-checks

#[kani::proof]
fn foo(x: i32) {
    if x > 5 {
        assert!(x < 4);
        if x < 3 {
            assert!(x == 2);
        }
    } else {
        assert!(x <= 5);
    }
}

fn main() {}
