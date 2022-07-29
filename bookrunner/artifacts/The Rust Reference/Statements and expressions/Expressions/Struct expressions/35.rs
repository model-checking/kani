// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
struct Point { x: f64, y: f64 }
struct NothingInMe { }
struct TuplePoint(f64, f64);
mod game { pub struct User<'a> { pub name: &'a str, pub age: u32, pub score: usize } }
struct Cookie; fn some_fn<T>(t: T) {}
Point {x: 10.0, y: 20.0};
NothingInMe {};
TuplePoint(10.0, 20.0);
TuplePoint { 0: 10.0, 1: 20.0 }; // Results in the same value as the above line
let u = game::User {name: "Joe", age: 35, score: 100_000};
some_fn::<Cookie>(Cookie);
}