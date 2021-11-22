// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
enum Foo {
    A(i32),
    B([i32; 0]),
}

fn get_none() -> Option<Foo> {
    None
}

fn get_a() -> Option<Foo> {
    Some(Foo::A(10))
}

fn get_b() -> Option<Foo> {
    Some(Foo::B([]))
}

fn main() {
    match get_none() {
        None => {}
        Some(_) => assert!(false),
    }

    match get_a() {
        Some(Foo::A(x)) => assert!(x == 10),
        _ => assert!(false),
    }

    match get_b() {
        Some(Foo::B(x)) => {}
        _ => assert!(false),
    }
}
