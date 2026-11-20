// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks the performance when adding in the Display trait.
//! The test is from https://github.com/model-checking/kani/issues/1996
//! With CBMC 5.79.0, all harnesses take ~3 seconds
//!
//! The input string and `kani::unwind` bound are deliberately minimal. The
//! `slow` harness exercises the `Display`/`to_string` formatting path, whose
//! SAT encoding is memory-heavy on newer CBMC: the original `"foo"`/unwind(6)
//! version peaked at ~26 GB on CBMC 6.10 and OOM-killed the 16 GB `perf`
//! runner. An empty payload (formatted result `"A."`) with the smallest unwind
//! that still fully unrolls the formatting loops keeps peak memory to ~11 GB
//! while still covering the trait/formatting path this test is meant to guard.
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
#[kani::unwind(3)]
fn fast() {
    let a = Foo::A(String::from(""));
    let s = match a {
        Foo::A(s) => format!("A.{s}"),
        Foo::B(s) => format!("B.{s}"),
    };
    assert_eq!(s, "A.");
}

#[kani::proof]
#[kani::unwind(3)]
fn slow() {
    let a = Foo::A(String::from(""));
    let s = a.to_string();
    assert_eq!(s, "A.");
}
