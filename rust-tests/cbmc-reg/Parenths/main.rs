// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let a = 10;
    let b = (a + 6) / 2;
    assert!(b == 8);
    let c = b * (a + 1);
    assert!(c == 88);

    let d = 55;
    let e = (b * (d + 8) + 1) * a;
    assert!(e == 10 * (500 + 5));
}
