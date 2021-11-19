// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::convert::TryInto;
fn main() {
    let input: &[u8] = &vec![
        unsafe { rmc::nondet() },
        unsafe { rmc::nondet() },
        unsafe { rmc::nondet() },
        unsafe { rmc::nondet() },
        unsafe { rmc::nondet() },
        unsafe { rmc::nondet() },
        unsafe { rmc::nondet() },
        unsafe { rmc::nondet() },
    ];
    let buffer = input.as_ref();
    let bytes: [u8; 8] = buffer.try_into().unwrap();
    let _value = u64::from_be_bytes(bytes);
}
