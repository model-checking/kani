// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness check -Z stubbing
//
//! This tests whether we provide a user friendly error if more than one harness has stubs

fn foo(b: bool) {
    assert!(b);
}

fn bar(b: bool) {
    assert!(!b);
}

/// Harness should succeed if stub has been applied and fail otherwise.
#[kani::proof]
#[kani::stub(foo, bar)]
fn check_stub_foo() {
    foo(false)
}

/// Harness should succeed if stub has been applied and fail otherwise.
#[kani::proof]
#[kani::stub(bar, foo)]
fn check_stub_bar() {
    bar(true)
}

#[kani::proof]
fn check_no_stub() {
    foo(true);
    bar(false);
}
