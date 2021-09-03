// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we properly handle structs for which the compiler reorders the
// fields to optimize the layout. In such cases, the field with minimum offset
// need not be the first field in the original struct (e.g. in "Foo" below, "b"
// is the field with minimum offset even though "a" is the leftmost field in the
// original struct).

enum E {
    Foo { a: u64, b: u16 },
    Bar,
}

fn main() {
    let e = E::Foo { a: 32, b: 100 };
    match e {
        E::Foo { a, b } => {
            assert!(a == 32);
            assert!(b == 100);
        }
        E::Bar => assert!(false),
    }
}
