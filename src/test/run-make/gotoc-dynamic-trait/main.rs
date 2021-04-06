// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
trait Shape {
    fn area(&self) -> u32;
    fn vol(&self, z: u32) -> u32;
    fn compare_area(&self, other: &dyn Shape) -> bool {
        self.area() > other.area()
    }
}

#[derive(Clone, Copy)]
struct Rectangle {
    w: u32,
    h: u32,
}
#[derive(Clone, Copy)]
struct Square {
    w: u32,
}

impl Shape for Rectangle {
    fn area(&self) -> u32 {
        self.w * self.h
    }
    fn vol(&self, z: u32) -> u32 {
        self.w * self.h * z
    }
}

impl Shape for Square {
    fn area(&self) -> u32 {
        self.w * self.w
    }
    fn vol(&self, z: u32) -> u32 {
        self.w * self.w * z
    }
}

fn do_something(x: &dyn Shape) -> u32 {
    x.area()
}
fn do_vol(x: &dyn Shape, z: u32) -> u32 {
    x.vol(z)
}

fn impl_area(a: impl Shape) -> u32 {
    a.area()
}

fn main() {
    let rec = Rectangle { w: 10, h: 5 };
    assert!(rec.vol(3) == 150);
    assert!(impl_area(rec.clone()) == 50);

    let vol = do_vol(&rec as &dyn Shape, 2);
    assert!(vol == 100);

    let square = Square { w: 3 };
    assert!(square.vol(3) == 27);
    assert!(do_vol(&square, 2) == 18);
    assert!(impl_area(square.clone()) == 9);

    assert!(!square.compare_area(&square));
    assert!(!square.compare_area(&rec));
    assert!(rec.compare_area(&square));
    assert!(!rec.compare_area(&rec));
}
