// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing --harness main
//
// Regression test: generic parameter names can differ between original and stub.
// Previously this was a fixme test for https://github.com/model-checking/kani/issues/1953

fn foo<T>(_x: T) -> bool {
    false
}

fn bar<S>(_x: S) -> bool {
    true
}

#[kani::proof]
#[kani::stub(foo, bar)]
pub fn main() {
    assert!(foo(42));
}
