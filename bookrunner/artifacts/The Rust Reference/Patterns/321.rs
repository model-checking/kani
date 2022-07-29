// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
let x = 20;
let (a, _) = (10, x);   // the x is always matched by _
assert_eq!(a, 10);

// ignore a function/closure param
let real_part = |a: f64, _: f64| { a };

// ignore a field from a struct
struct RGBA {
   r: f32,
   g: f32,
   b: f32,
   a: f32,
}
let color = RGBA{r: 0.4, g: 0.1, b: 0.9, a: 0.5};
let RGBA{r: red, g: green, b: blue, a: _} = color;
assert_eq!(color.r, red);
assert_eq!(color.g, green);
assert_eq!(color.b, blue);

// accept any Some, with any value
let x = Some(10);
if let Some(_) = x {}
}