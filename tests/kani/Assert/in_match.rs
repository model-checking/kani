// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test verifies that using the assert macro inside a match arm works

enum Foo {
    A,
    B,
}

#[kani::proof]
fn check_assert_in_match() {
    let f = Foo::A;
    match f {
        Foo::A => assert!(1 + 1 == 2, "Message"),
        Foo::B => panic!("Failed"),
    }
}
