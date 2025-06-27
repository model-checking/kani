// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that we can codegen structs with scalar and ZSTs.

struct Empty {}

pub struct Foo {
    x: u8,
    _t: Empty,
}

#[kani::proof]
fn check_zst() {
    const C: Foo = Foo { x: 0, _t: Empty {} };
    assert_eq!(C.x, 0);
}
