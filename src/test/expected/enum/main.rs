// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
enum Foo {
    A(i32),
    B { x: i32, y: f64 },
}

fn a() -> Foo {
    Foo::A(10)
}

fn b() -> Foo {
    Foo::B { x: 30, y: 60.0 }
}

fn main() {
    let x = a();
    match x {
        Foo::A(x) => assert!(x == 10),
        Foo::B { .. } => assert!(false),
    }
    match b() {
        Foo::A(_) => assert!(false),
        Foo::B { x, y } => {
            assert!(x == 30 && y == 60.0);
            assert!(x == 31);
        }
    }
}
