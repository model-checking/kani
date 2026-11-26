// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub trait Shape {
    fn area(&self) -> u32;
}

pub struct Square(pub u32);

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0.wrapping_mul(self.0)
    }
}

/// Dynamic dispatch: the callee is not known statically.
pub fn total_area(shapes: &[&dyn Shape]) -> u32 {
    let mut total = 0u32;
    for s in shapes {
        total = total.wrapping_add(s.area());
    }
    total
}

/// A generic function.
pub fn largest<T: PartialOrd + Copy>(a: T, b: T) -> T {
    if a > b { a } else { b }
}
