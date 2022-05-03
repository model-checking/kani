// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Dynamic traits should work when used through a box
// _fail test; all assertions inverted.
trait Shape {
    fn area(&self) -> u32;
    fn vol(&self, z: u32) -> u32;
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

fn do_vol(x: &dyn Shape, z: u32) -> u32 {
    x.vol(z)
}

fn do_area_box(x: Box<dyn Shape>) -> u32 {
    x.area()
}

#[kani::proof]
fn main() {
    let rec = Box::new(Rectangle { w: 10, h: 5 });
    assert!(rec.vol(3) != 150);
    assert!(do_vol(&*rec, 2) != 100);
    assert!(do_area_box(rec) != 50);

    let square = Box::new(Square { w: 3 });
    assert!(square.vol(3) != 27);
    assert!(do_vol(&*square, 2) != 18);
}
