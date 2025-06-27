// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use kani::Arbitrary;

struct Foo {
    x: i32,
    y: i32,
    z: i32,
}

impl Arbitrary for Foo {
    fn any() -> Self {
        Foo { x: 3, y: 4, z: 5 }
    }
}

fn main() {
    let f: Foo = kani::any();
    assert_eq!(f.x, 3);
    assert_eq!(f.y, 4);
    assert_eq!(f.z, 5);
}
