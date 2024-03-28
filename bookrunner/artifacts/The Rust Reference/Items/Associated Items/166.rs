// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
type Surface = i32;
type BoundingBox = i32;
trait Shape {
    fn draw(&self, surface: Surface);
    fn bounding_box(&self) -> BoundingBox;
}

struct Circle {
    // ...
}

impl Shape for Circle {
    // ...
  fn draw(&self, _: Surface) {}
  fn bounding_box(&self) -> BoundingBox { 0i32 }
}

impl Circle {
    fn new() -> Circle { Circle{} }
}

let circle_shape = Circle::new();
let bounding_box = circle_shape.bounding_box();
}