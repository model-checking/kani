// Copyright rustc Contributors
// SPDX-License-Identifier: Apache OR MIT
// Adapted from rustc: src/test/ui/generator/overlap-locals.rs
// Changes: copyright Kani contributors, Apache or MIT

// run-pass

#![feature(generators)]

#[kani::proof]
fn main() {
    let a = || {
        {
            let w: i32 = 4;
            yield;
            println!("{:?}", w);
        }
        {
            let x: i32 = 5;
            yield;
            println!("{:?}", x);
        }
        {
            let y: i32 = 6;
            yield;
            println!("{:?}", y);
        }
        {
            let z: i32 = 7;
            yield;
            println!("{:?}", z);
        }
    };
    assert_eq!(8, std::mem::size_of_val(&a));
}
