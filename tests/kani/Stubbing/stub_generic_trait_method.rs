// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! Test stubbing generic trait method implementations.
//! Regression test for https://github.com/model-checking/kani/issues/1997

trait Convert<T> {
    fn convert(&self) -> T;
}

struct MyType;

impl Convert<u32> for MyType {
    fn convert(&self) -> u32 {
        100
    }
}

impl Convert<bool> for MyType {
    fn convert(&self) -> bool {
        false
    }
}

fn stub_convert_u32(_x: &MyType) -> u32 {
    42
}

#[kani::proof]
#[kani::stub(<MyType as Convert<u32>>::convert, stub_convert_u32)]
fn check_generic_trait_stub() {
    let m = MyType;
    assert_eq!(<MyType as Convert<u32>>::convert(&m), 42);
    // The bool impl should NOT be affected
    assert_eq!(<MyType as Convert<bool>>::convert(&m), false);
}
