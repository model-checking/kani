// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --harness empty_harness

// This file is to be used as a module on a different test, but the compiletest will still run
// kani on this file. Use an empty harness instead.

pub fn not_empty(v: &[i32]) {
    assert!(!v.is_empty());
}

#[kani::proof]
fn empty_harness() {
    // No-op to overcome compile test.
}
