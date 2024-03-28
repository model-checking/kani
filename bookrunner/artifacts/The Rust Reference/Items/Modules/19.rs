// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
mod math {
    type Complex = (f64, f64);
    fn sin(f: f64) -> f64 {
        /* ... */
      unimplemented!();
    }
    fn cos(f: f64) -> f64 {
        /* ... */
      unimplemented!();
    }
    fn tan(f: f64) -> f64 {
        /* ... */
      unimplemented!();
    }
}
}