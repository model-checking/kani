// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check the message printed when a checked operation fails.
// kani-flags: --unwind 3
extern crate kani;

use kani::any;

#[kani::proof]
#[kani::proof]
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
