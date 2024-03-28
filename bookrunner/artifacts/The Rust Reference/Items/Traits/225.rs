// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
trait Shape { fn area(&self) -> f64; }
trait Circle : Shape { fn radius(&self) -> f64; }
struct UnitCircle;
impl Shape for UnitCircle { fn area(&self) -> f64 { std::f64::consts::PI } }
impl Circle for UnitCircle { fn radius(&self) -> f64 { 1.0 } }
let circle = UnitCircle;
let circle = Box::new(circle) as Box<dyn Circle>;
let nonsense = circle.radius() * circle.area();
}