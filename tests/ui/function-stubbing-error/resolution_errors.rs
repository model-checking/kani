// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! This tests that we emit a nice error message for resolution failures.

/// Dummy structure
pub struct Bar;

/// Dummy stub
pub fn stub_foo() -> bool {
    true
}

#[kani::proof]
#[kani::stub(<Bar>::foo, stub_foo)]
#[kani::stub(u8::foo, stub_foo)]
#[kani::stub(<(i32, i32)>::foo, stub_foo)]
#[kani::stub(<[u32]>::foo, stub_foo)]
#[kani::stub(str::foo, stub_foo)]
#[kani::stub(<[char; 10]>::foo, stub_foo)]
fn invalid_methods() {}
