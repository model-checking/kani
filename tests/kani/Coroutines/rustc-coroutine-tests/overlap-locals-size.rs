// Copyright rustc Contributors
// Adapted from rustc: https://github.com/rust-lang/rust/tree/5f98537eb7b5f42c246a52c550813c3cff336069/src/test/ui/coroutine/overlap-locals.rs
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// run-pass

#![feature(coroutines)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn main() {
    let a = #[coroutine]
    || {
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

    // FIXME: size of coroutines does not work reliably (https://github.com/model-checking/kani/issues/1395)
    assert_eq!(8, std::mem::size_of_val(&a));
}
