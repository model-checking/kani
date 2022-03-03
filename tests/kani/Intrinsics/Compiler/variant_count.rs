// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Check that `variant_count` is not supported.
// This test can be replaced with `variant_count_fixme.rs` once the intrinsic is
// supported and works as expected.

#![feature(variant_count)]
use std::mem;

enum Void {}

fn main() {
    let _ = mem::variant_count::<Void>();
}
