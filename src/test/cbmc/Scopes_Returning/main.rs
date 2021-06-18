// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
include!("../../rmc-prelude.rs");

fn main() {
    let x = { 5 };
    assert!(x == 5);

    let a = {
        let mut b = 3;
        b *= 3;
        b
    };
    assert!(a == 9);

    let c = {
        let mut c = 3;
        c *= 3;
        c + 1
    };
    assert!(c == 10);

    let d: u32 = __nondet();
    let e = {
        let f: u32;

        if d < 10 {
            f = d;
        } else {
            f = 10;
        }

        f
    };
    assert!(e == d || e == 10);

    let g: u32 = __nondet();
    let h = {
        if g < 10 { g } else { 10 }
    };
    assert!(h == g || h == 10);
}
