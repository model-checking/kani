// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! This tests stubbing for methods in local enums.

enum LocalType {
    Empty,
}

impl LocalType {
    pub fn new() -> Self {
        Self::Empty
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

#[kani::proof]
#[kani::stub(LocalType::pub_fn, LocalType::the_answer)]
fn check_enum_pub_method_stub() {
    assert_eq!(LocalType::new().pub_fn(), 42);
}

#[kani::proof]
#[kani::stub(LocalType::priv_fn, LocalType::the_answer)]
fn check_enum_priv_method_via_delegation() {
    assert_eq!(LocalType::new().fn_delegating_to_priv_fn(), 42);
}
