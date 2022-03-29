// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that enums with negative discriminants are handled correctly

enum Foo {
    A = -500,
    B = -200,
    C = -100,
    D = 0,
    E = 1,
    F = 256,
}

#[kani::proof]
fn main() {
    let a = Some(Foo::A);
    let b = Some(Foo::B);
    let c = Some(Foo::C);
    let d = Some(Foo::D);
    let e = Some(Foo::E);
    let f = Some(Foo::F);
    let _ = matches!(a, Some(Foo::A));
    let _ = matches!(b, Some(Foo::B));
    let _ = matches!(c, Some(Foo::C));
    let _ = matches!(d, Some(Foo::D));
    let _ = matches!(e, Some(Foo::E));
    let _ = matches!(f, Some(Foo::F));
}
