// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that derive BoundedArbitrary macro works on enums with a single variant
//! See https://github.com/model-checking/kani/issues/4170

#[allow(unused)]
#[derive(kani::BoundedArbitrary)]
enum Foo {
    A(#[bounded] String),
}

#[kani::proof]
#[kani::unwind(6)]
fn check_enum() {
    let any_enum: Foo = kani::bounded_any::<_, 4>();
    let Foo::A(s) = any_enum;
    kani::cover!(s.len() == 0);
    kani::cover!(s.len() == 1);
    kani::cover!(s.len() == 2);
    kani::cover!(s.len() == 3);
    kani::cover!(s.len() == 4);
}
