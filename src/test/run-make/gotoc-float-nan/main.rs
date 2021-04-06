// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let mut f: f32 = 2.0;
    while f != 0.0 {
        f -= 1.0;
    }

    // at this point, f == 0.0
    // should succeed
    assert!(1.0 / f != 0.0 / f);
    // should succeed
    assert!(!(1.0 / f == 0.0 / f));
    // should fail
    assert!(1.0 / f == 0.0 / f);
    // should fail
    assert!(0.0 / f == 0.0 / f);
    // should suceed
    assert!(0.0 / f != 0.0 / f);
}
