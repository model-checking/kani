// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks the performance when adding in the Display trait.
//! The test is from https://github.com/model-checking/kani/issues/1996
//! With CBMC 5.79.0, all harnesses take ~3 seconds
use std::fmt::Display;

enum Foo {
    A(String),
    B(String),
}

impl Display for Foo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Foo::A(s) => format!("A.{s}"),
            Foo::B(s) => format!("B.{s}"),
        };
        write!(f, "{s}")?;
        Ok(())
    }
}

#[kani::proof]
#[kani::unwind(6)]
fn fast() {
    let a = Foo::A(String::from("foo"));
    let s = match a {
        Foo::A(s) => format!("A.{s}"),
        Foo::B(s) => format!("B.{s}"),
    };
    assert_eq!(s, "A.foo");
}

#[kani::proof]
#[kani::unwind(6)]
fn slow() {
    let a = Foo::A(String::from("foo"));
    let s = a.to_string();
    assert_eq!(s, "A.foo");
}
