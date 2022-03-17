// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check we don't print temporary variables as part of CBMC messages.
// cbmc-flags: --signed-overflow-check
extern crate kani;

use kani::any;

// Ensure rustc encodes the operation.
fn dummy(var: i32) {
    kani::assume(var != 0);
}

#[kani::proof]
fn main() {
    dummy(any::<i32>() + any::<i32>());
    dummy(any::<i32>() - any::<i32>());
    dummy(any::<i32>() * any::<i32>());
    dummy(any::<i32>() / any::<i32>()); // This is not emitting CBMC check.
    dummy(any::<i32>() % any::<i32>()); // This is not emitting CBMC check.
    dummy(any::<i32>() << any::<i32>());
    dummy(any::<i32>() >> any::<i32>());
    dummy(-any::<i32>()); // This is not emitting CBMC check.
}

