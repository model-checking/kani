// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing --harness main
//
// We currently require a stub and the original/function method to have the same
// names for generic parameters; instead, we should allow for renaming.
// See <https://github.com/model-checking/kani/issues/1953> for more information.

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
