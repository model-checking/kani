// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that Kani's overridden versions of the print macros do not
// take ownership of variables passed as arguments

#[derive(Debug)]
struct Foo {
    x: i32,
}

#[kani::proof]
fn main() {
    let foo = Foo { x: 5 };
    // calling `println` with `foo` should not move it
    println!("{:?}", foo);
    // make sure reading `foo` does not produce a "use of moved value" error
    let y = foo.x;
    assert!(y == 5);
}
