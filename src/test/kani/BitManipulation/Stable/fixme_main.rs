// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-verify-fail

fn main() {
    // Intrinsics implemented as integer primitives
    // https://doc.rust-lang.org/core/intrinsics/fn.cttz.html
    // https://doc.rust-lang.org/core/intrinsics/fn.ctlz.html
    // https://doc.rust-lang.org/std/intrinsics/fn.rotate_left.html
    let x = 0b0011_1000_u8;
    let num_trailing = x.trailing_zeros();
    let num_leading = x.leading_zeros();
    let rotated_num = x.rotate_left(3);

    assert!(num_trailing == 3); // fails because of https://github.com/model-checking/rmc/issues/26
    assert!(num_leading == 2);
    assert!(rotated_num == 0b1100_0001_u8);
}
