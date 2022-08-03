// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check the message printed when a checked operation fails.
extern crate kani;

use kani::any;

#[kani::proof]
#[kani::unwind(4)]
fn main() {
    let _ = any::<u8>() + any::<u8>();
    let _ = any::<u8>() - any::<u8>();
    let _ = any::<u8>() * any::<u8>();
    let _ = any::<u8>() / any::<u8>();
    let _ = any::<u8>() % any::<u8>();
    let _ = any::<u8>() << any::<u8>();
    let _ = any::<u8>() >> any::<u8>();
    let _ = -any::<i8>();
    let _ = kani::any::<[u8; 2]>()[any::<usize>()];
}
