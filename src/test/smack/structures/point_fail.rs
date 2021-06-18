// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @expect error

use std::ops::{Add, AddAssign};

#[derive(PartialEq, Clone, Copy)]
struct Point {
    x: u64,
    y: u64,
}

impl Point {
    pub fn new(_x: u64, _y: u64) -> Point {
        Point { x: _x, y: _y }
    }
    pub fn get_x(self) -> u64 {
        self.x
    }
    pub fn get_y(self) -> u64 {
        self.y
    }
}

impl Add for Point {
    type Output = Point;
    fn add(self, other: Point) -> Point {
        Point::new(self.x + other.x, self.y + other.y)
    }
}

impl AddAssign for Point {
    fn add_assign(&mut self, other: Point) {
        self.x += other.x;
        self.y += other.y;
    }
}

include!("../../rmc-prelude.rs");

pub fn main() {
    let w = __nondet();
    let x = __nondet();
    let y = __nondet();
    let z = __nondet();

    if w <= std::u64::MAX / 2 // avoid overflow
        && x <= std::u64::MAX / 2 // avoid overflow
        && y <= std::u64::MAX / 2 // avoid overflow
        && z <= std::u64::MAX / 2
    {
        // avoid overflow

        let a = Point::new(w, x);
        let b = Point::new(y, z);
        let c = a + b;
        assert!(c != Point::new(w + y, x + z));
        assert!(c == Point::new(a.get_x() + b.get_x(), a.get_y() + b.get_y()));
    }
}
