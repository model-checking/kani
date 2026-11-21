// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This tests that stubs are applied to calls the MIR inliner would inline when
//! building with cargo.

fn target() -> u32 {
    1
}

fn caller() -> u32 {
    target()
}

fn stub() -> u32 {
    2
}

#[kani::proof]
#[kani::stub(target, stub)]
fn check_stub_inlined_function() {
    assert_eq!(caller(), 2);
}
