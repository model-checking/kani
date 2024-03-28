// compile-flags: --edition 2015
#![allow(unused)]
#![allow(incomplete_features)]
#![feature(unsized_locals, unsized_fn_params)]

use std::any::Any;

fn main() {
    let x: Box<dyn Any> = Box::new(42);
    let x: dyn Any = *x;
    //  ^ unsized local variable
    //               ^^ unsized temporary
    foo(x);
}

fn foo(_: dyn Any) {}
//     ^^^^^^ unsized argument