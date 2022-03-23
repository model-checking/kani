// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check we don't print temporary variables as part of CBMC messages.
extern crate kani;

use kani::any;

// Use the result so rustc doesn't optimize them away.
fn dummy(result: f32) -> f32 {
    result
}

#[kani::proof]
fn main() {
    dummy(any::<f32>() + any::<f32>());
    dummy(any::<f32>() - any::<f32>());
    dummy(any::<f32>() * any::<f32>());
    dummy(any::<f32>() / any::<f32>()); // This is not emitting CBMC check.
    dummy(any::<f32>() % any::<f32>()); // This is not emitting CBMC check.
    dummy(-any::<f32>()); // This is not emitting CBMC check.
}

