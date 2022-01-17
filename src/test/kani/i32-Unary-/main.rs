// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let a: i32 = kani::any();
    if -100000 < a && a < 100000 {
        let b = -a;

        if a > 0 {
            assert!(a > b);
        } else if a < 0 {
            assert!(a < b - 1);
        } else {
            assert!(b == a);
        }
    }
}
