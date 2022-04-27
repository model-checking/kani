// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that an offset that overflows an `isize::MAX` triggers a verification failure.
use std::convert::TryInto;

#[kani::proof]
unsafe fn check_wrap_offset() {
    let v: &[u128] = &[0; 200];
    let v_100: *const u128 = &v[100];
    let max_offset = usize::MAX / std::mem::size_of::<u128>();
    let v_wrap: *const u128 = v_100.offset((max_offset + 1).try_into().unwrap());
    assert_eq!(v_100, v_wrap);
    assert_eq!(*v_100, *v_wrap);
}
