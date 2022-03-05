// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

fn main() {
    // Intrinsics implemented as integer primitives
    // https://doc.rust-lang.org/core/intrinsics/fn.cttz.html
    // https://doc.rust-lang.org/core/intrinsics/fn.ctlz.html
    let x = 0b0011_1000_u8;
    let num_trailing = x.trailing_zeros();
    let num_leading = x.leading_zeros();

    assert!(num_trailing == 3); // fails because of https://github.com/model-checking/kani/issues/26
    assert!(num_leading == 2);
}
