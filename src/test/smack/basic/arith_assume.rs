// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @expect verified

pub fn main() {
    let a = rmc::any();
    let b = rmc::any();
    if 4 < a && a < 8 {
        // a in [5,7]
        if 5 < b && b < 9 {
            // b in [6,8]
            assert!(30 <= a * b && a * b <= 56); // a*b in [30,56]
        }
    }
}
