// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
struct Foo<T> {
    data: T,
    i: i32,
}

fn ident<T>(x: T) -> T {
    x
}

fn wrapped<T>(x: T) -> Foo<T> {
    Foo { data: ident(x), i: 0 }
}

fn main() {
    let x = 10;
    let y = wrapped(x);
    let z = 20.0;
    let w = wrapped(z);
    assert!(x == y.data);
    assert!(z == w.data);
}
