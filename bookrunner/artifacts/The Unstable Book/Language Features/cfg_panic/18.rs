// compile-flags: --edition 2015
#![allow(unused)]
#![feature(cfg_panic)]

fn main() {
#[cfg(panic = "unwind")]
fn a() {
    // ...
}

#[cfg(not(panic = "unwind"))]
fn a() {
    // ...
}

fn b() {
    if cfg!(panic = "abort") {
        // ...
    } else {
        // ...
    }
}
}