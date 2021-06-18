// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
include!("../../rmc-prelude.rs");

fn main() {
    let mut x = 0;
    x |= 1;
    assert!(x == 1);
    x ^= 7;
    assert!(x == 6);
    x %= 4;
    assert!(x == 2);
    x = 18;
    x &= 15;
    assert!(x == 2);

    let mut a: u32 = __nondet();
    a %= 8;

    let mut b: u32 = __nondet();
    b %= 8;

    let mut c = a;
    let mut d = b;

    c &= b;
    d |= a;

    assert!(c < 8 && d < 8);
    assert!(c + d >= c & d);
}
