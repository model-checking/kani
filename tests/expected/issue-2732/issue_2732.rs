// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

const C: [u32; 5] = [0; 5];

#[allow(unconditional_panic)]
fn test() -> u32 {
    C[10]
}

#[kani::proof]
fn main() {
    test();
}
