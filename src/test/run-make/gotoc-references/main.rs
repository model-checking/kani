// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
struct Foo {
    a: i32,
    _b: f64,
}

impl Foo {
    pub fn get_a(&self) -> i32 {
        self.a
    }
}

fn main() {
    let foo = Foo { a: 2, _b: 3.0 };
    let z = foo.get_a();
    assert!(z == 2);
}
