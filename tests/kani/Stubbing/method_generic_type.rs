// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness main -Z stubbing
//
//! This tests stubbing for methods of a local type that has generic parameters.

struct LocalType<T> {
    _x: std::marker::PhantomData<T>,
}

impl<T> LocalType<T> {
    pub fn new() -> Self {
        Self { _x: std::marker::PhantomData }
    }

    pub fn pub_fn(&self) -> u32 {
        0
    }

    pub fn fn_delegating_to_priv_fn(&self) -> u32 {
        self.priv_fn()
    }

    fn priv_fn(&self) -> u32 {
        0
    }

    fn the_answer(&self) -> u32 {
        42
    }
}

// TODO: Split up these assertions into separate harnesses, once stubbing is able to support that.
// <https://github.com/model-checking/kani/issues/1861>
#[kani::proof]
#[kani::stub(LocalType::pub_fn, LocalType::the_answer)]
#[kani::stub(LocalType::priv_fn, LocalType::the_answer)]
fn main() {
    assert_eq!(LocalType::<i32>::new().pub_fn(), 42);
    assert_eq!(LocalType::<String>::new().fn_delegating_to_priv_fn(), 42);
}
