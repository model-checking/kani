// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::convert::TryInto;

include!("../../rmc-prelude.rs");

fn main() {
    let input: &[u8] = &vec![
        __nondet(),
        __nondet(),
        __nondet(),
        __nondet(),
        __nondet(),
        __nondet(),
        __nondet(),
        __nondet(),
    ];
    let buffer = input.as_ref();
    let bytes: [u8; 8] = buffer.try_into().unwrap();
    let value = u64::from_be_bytes(bytes);
    let idx: usize = __nondet();
    if idx < 8 {
        assert!(u64::to_be_bytes(value)[idx] == input[idx]);
    }
}
