// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @expect error
// kani-verify-fail

pub fn main() {
    let a = kani::any();
    let b = kani::any();
    if 4 < a && a < 8 {
        // a in [5,7]
        if 5 < b && b < 9 {
            // b in [6,8]
            let x = a * b;
            assert!(
                !(x == 30
                    || x == 35
                    || x == 40
                    || x == 36
                    || x == 48
                    || x == 42
                    || x == 49
                    || x == 56)
            ); // a*b != anything allowed
        }
    }
}
