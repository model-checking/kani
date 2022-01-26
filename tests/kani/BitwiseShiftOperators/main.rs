// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    assert!(8 >> 4 == 0);
    assert!(1 << 4 == 16);

    let mut a = 8;
    assert!(a >> 1 == 4);
    assert!(a << 1 == 16);

    a <<= 2;
    assert!(a == 32);
    a >>= 3;
    assert!(a == 4);
}
