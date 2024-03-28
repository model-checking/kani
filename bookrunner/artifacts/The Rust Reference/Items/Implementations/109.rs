// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
#[derive(Copy, Clone)]
struct Point {x: f64, y: f64};
type Surface = i32;
struct BoundingBox {x: f64, y: f64, width: f64, height: f64};
trait Shape { fn draw(&self, s: Surface); fn bounding_box(&self) -> BoundingBox; }
fn do_draw_circle(s: Surface, c: Circle) { }
struct Circle {
    radius: f64,
    center: Point,
}

impl Copy for Circle {}

impl Clone for Circle {
    fn clone(&self) -> Circle { *self }
}

impl Shape for Circle {
    fn draw(&self, s: Surface) { do_draw_circle(s, *self); }
    fn bounding_box(&self) -> BoundingBox {
        let r = self.radius;
        BoundingBox {
            x: self.center.x - r,
            y: self.center.y - r,
            width: 2.0 * r,
            height: 2.0 * r,
        }
    }
}
}