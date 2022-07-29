// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
struct Point {
    x: u32,
    y: u32,
}
let s = Point {x: 1, y: 1};

match s {
    Point {x: 10, y: 20} => (),
    Point {y: 10, x: 20} => (),    // order doesn't matter
    Point {x: 10, ..} => (),
    Point {..} => (),
}

struct PointTuple (
    u32,
    u32,
);
let t = PointTuple(1, 2);

match t {
    PointTuple {0: 10, 1: 20} => (),
    PointTuple {1: 10, 0: 20} => (),   // order doesn't matter
    PointTuple {0: 10, ..} => (),
    PointTuple {..} => (),
}
}