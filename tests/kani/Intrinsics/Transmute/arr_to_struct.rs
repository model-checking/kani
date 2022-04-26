// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `transmute` works as expected when turning an array into
// a struct.

struct Pair {
    fst: u16,
    snd: u16,
}

#[kani::proof]
fn main() {
    let arr = [0; 4];
    let pair = unsafe { std::mem::transmute::<[u8; 4], Pair>(arr) };
    assert_eq!(pair.fst, 0);
    assert_eq!(pair.snd, 0);
}
