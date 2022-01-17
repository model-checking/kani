// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

extern "C" {
    pub fn external_c_assertion(i: u32) -> u32;
}

#[no_mangle]
pub extern "C" fn rust_add1(i: u32) -> u32 {
    i + 1
}
