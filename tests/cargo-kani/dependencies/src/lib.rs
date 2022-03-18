// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
pub fn check_dummy() {
    let x = kani::any::<u8>();
    kani::assume(x > 10);
    assert!(x > 2);
}
