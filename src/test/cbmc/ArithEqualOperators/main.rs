// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
include!("../../rmc-prelude.rs");

fn main() {
    let mut a: u32 = __nondet();
    a /= 2;
    let mut b: u32 = __nondet();
    b /= 2;
    let c = b;
    b += a;

    let d = a;

    assert!(b > a || a == 0 || c == 0);

    b -= a;

    assert!(c == b);

    a *= 2;

    assert!(a > d || d == 0);
}
