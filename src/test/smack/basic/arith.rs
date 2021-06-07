// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @expect verified

pub fn main() {
    // unsigned
    {
        let a: u32 = 2;
        let b: u32 = 3;
        {
            let c = a + b;
            assert!(c == 5);
        }
        {
            let c = a * b;
            assert!(c == 6);
        }
        {
            let c = b - a;
            assert!(c == 1);
        }
        {
            let c = a % b;
            assert!(c == 2);
            let d = b % a;
            assert!(d == 1);
        }
        {
            let c = a / b;
            assert!(c == 0);
            let d = b / a;
            assert!(d == 1);
        }
    }
    // signed
    {
        let a: i32 = -3;
        let b: i32 = 5;
        {
            let c = a + b;
            assert!(c == 2);
        }
        {
            let c = a * b;
            assert!(c == -15);
        }
        {
            let c = b - a;
            assert!(c == 8);
        }
    }
}
