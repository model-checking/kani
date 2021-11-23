// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-flags: --cbmc-args --unwind 11
// rmc-verify-fail

// ANCHOR: code
fn initialize_prefix(length: usize, buffer: &mut [u8]) {
    // Let's just ignore invalid calls
    if length > buffer.len() {
        return;
    }

    for i in 0..=length {
        buffer[i] = 0;
    }
}
// ANCHOR_END: code

// ANCHOR: rmc
#[cfg(rmc)]
#[no_mangle]
fn main() {
    const LIMIT: usize = 10;
    let mut buffer: [u8; LIMIT] = [1; LIMIT];

    let length = rmc::nondet();
    rmc::assume(length <= LIMIT);

    initialize_prefix(length, &mut buffer);
}
// ANCHOR_END: rmc
