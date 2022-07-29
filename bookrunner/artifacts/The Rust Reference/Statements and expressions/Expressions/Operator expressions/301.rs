// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
fn sum(values: &[f64]) -> f64 { 0.0 }
fn len(values: &[f64]) -> i32 { 0 }
fn average(values: &[f64]) -> f64 {
    let sum: f64 = sum(values);
    let size: f64 = len(values) as f64;
    sum / size
}
}