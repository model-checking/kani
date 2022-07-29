// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
struct Point3d { x: i32, y: i32, z: i32 }
let x = 0;
let y_value = 0;
let z = 0;
Point3d { x: x, y: y_value, z: z };
Point3d { x, y: y_value, z };
}