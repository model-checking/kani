// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @expect reachable
// rmc-verify-fail

fn safe_div(x: u32, y: u32) -> Option<u32> {
    if y != 0 { Some(x / y) } else { None }
}

pub fn main() {
    let x = rmc::any();
    if x > 0 && x <= 100 {
        // avoid overflow
        let a = safe_div(2 * x, x);
        match a {
            Some(c) => assert!(c == 2),
            None => assert!(false),
        };
        let y = rmc::any();
        let b = safe_div(x, y);
        match b {
            Some(c) => assert!(c == x / y),
            None => assert!(false), // Division by zero should return None
        };
    }
}
