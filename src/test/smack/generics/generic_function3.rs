// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @expect error
// rmc-verify-fail

struct Point<T> {
    pub x: T,
    pub y: T,
}

struct Point3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

trait S<T> {
    fn swap_items(self) -> Self;
}

impl<T> S<T> for Point<T> {
    fn swap_items(self) -> Point<T> {
        Point::<T> { x: self.y, y: self.x }
    }
}

impl<T> S<T> for Point3<T> {
    fn swap_items(self) -> Point3<T> {
        Point3::<T> { x: self.y, y: self.z, z: self.x }
    }
}

fn swapem<T, U: S<T>>(s: U) -> U {
    s.swap_items()
}

include!("../../rmc-prelude.rs");

pub fn main() {
    let x2 = __nondet();
    let y2 = __nondet();
    let x3 = __nondet();
    let y3 = __nondet();
    let z3 = __nondet();
    let p2 = Point::<i64> { x: x2, y: y2 };
    let p3 = Point3::<i64> { x: x3, y: y3, z: z3 };

    let q2 = swapem(p2);
    let q3 = swapem(p3);
    assert!(q2.x == y2);
    assert!(q2.y == x2);
    assert!(q3.x != y3);
    assert!(q3.y == z3);
    assert!(q3.z == x3);
}
