// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::convert::TryInto;

fn main() {
    let input: &[u8] = &vec![
        rmc::any(),
        rmc::any(),
        rmc::any(),
        rmc::any(),
        rmc::any(),
        rmc::any(),
        rmc::any(),
        rmc::any(),
    ];
    let buffer = input.as_ref();
    let bytes: [u8; 8] = buffer.try_into().unwrap();
    let value = u64::from_be_bytes(bytes);
    let idx: usize = rmc::any();
    if idx < 8 {
        assert!(u64::to_be_bytes(value)[idx] == input[idx]);
    }
}
