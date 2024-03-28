// compile-flags: --edition 2015
#![allow(unused)]
#![feature(trace_macros)]

fn main() {
    trace_macros!(true);
    println!("Hello, Rust!");
    trace_macros!(false);
}