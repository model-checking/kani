// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that CBMC (as invoked via Kani) does not spuriously fail with arrays of more 64 elements.

#[derive(PartialEq, Eq)]
enum Foo {
    A,
    B([u8; 65]),
}

#[kani::proof]
fn main() {
    let x: Foo = Foo::B([42; 65]);
    let y: Foo = Foo::B([42; 65]);
    assert!(x == y);
}
