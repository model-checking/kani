// Copyright Kani Contributors
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
fn check_negative_discriminant() {
    let a = Some(Foo::A);
    let b = Some(Foo::B);
    let c = Some(Foo::C);
    let d = Some(Foo::D);
    let e = Some(Foo::E);
    let f = Some(Foo::F);
    let _ = assert!(matches!(a, Some(Foo::A)));
    let _ = assert!(matches!(b, Some(Foo::B)));
    let _ = assert!(matches!(c, Some(Foo::C)));
    let _ = assert!(matches!(d, Some(Foo::D)));
    let _ = assert!(matches!(e, Some(Foo::E)));
    let _ = assert!(matches!(f, Some(Foo::F)));
}
