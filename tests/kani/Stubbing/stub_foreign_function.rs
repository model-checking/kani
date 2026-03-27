// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! Test that foreign functions declared in `extern "C"` blocks can be stubbed.
//! Regression test for https://github.com/model-checking/kani/issues/2686

extern "C" {
    fn foreign_fn() -> u32;
}

fn foreign_fn_stub() -> u32 {
    42
}

#[kani::proof]
#[kani::stub(foreign_fn, foreign_fn_stub)]
fn check_foreign_fn_stub() {
    let result = unsafe { foreign_fn() };
    assert_eq!(result, 42);
}
