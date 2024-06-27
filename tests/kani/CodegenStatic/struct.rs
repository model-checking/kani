// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// This is a custom type which is parameterized by a `usize`
pub struct Foo<const N: usize> {
    bytes: [u8; N],
}

const x: Foo<3> = Foo { bytes: [1, 2, 3] };

#[kani::proof]
fn simple_struct() {
    assert!(x.bytes[0] == 1);
}

pub struct Outer {
    data: char,
    inner: Inner,
}

pub struct Inner {
    a: char,
    b: char,
    c: char,
}

static OUTER: Outer = Outer { data: 'a', inner: Inner {a: 'a', b: 'b', c: 'c' } };

#[kani::proof]
fn nested_struct() {
    assert!(OUTER.inner.c == 'c');
}
