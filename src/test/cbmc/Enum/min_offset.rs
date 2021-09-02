// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
enum E {
    Foo { a: u64, b: u16, },
    Bar,
}

fn main() {
    let e = E::Foo { a: 32, b: 100 };
    match e {
        E::Foo{ a, b } => {
            assert!(a == 32);
            assert!(b == 100);
        }
        E::Bar => assert!(false),
    }
}