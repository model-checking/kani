// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[no_mangle]
pub fn check_dummy() {
    let x = unsafe { rmc::nondet::<u8>() };
    rmc::assume(x > 10);
    assert!(x > 2);
}
