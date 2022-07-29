// kani-check-fail
// compile-flags: --edition 2015
#![allow(unused)]
#![feature(unsized_locals)]

use std::any::Any;

struct MyStruct<T: ?Sized> {
    content: T,
}

struct MyTupleStruct<T: ?Sized>(T);

fn answer() -> Box<dyn Any> {
    Box::new(42)
}

fn main() {
    // You CANNOT have unsized statics.
    static X: dyn Any = *answer();  // ERROR
    const Y: dyn Any = *answer();  // ERROR

    // You CANNOT have struct initialized unsized.
    MyStruct { content: *answer() };  // ERROR
    MyTupleStruct(*answer());  // ERROR
    (42, *answer());  // ERROR

    // You CANNOT have unsized return types.
    fn my_function() -> dyn Any { *answer() }  // ERROR

    // You CAN have unsized local variables...
    let mut x: dyn Any = *answer();  // OK
    // ...but you CANNOT reassign to them.
    x = *answer();  // ERROR

    // You CANNOT even initialize them separately.
    let y: dyn Any;  // OK
    y = *answer();  // ERROR

    // Not mentioned in the RFC, but by-move captured variables are also Sized.
    let x: dyn Any = *answer();
    (move || {  // ERROR
        let y = x;
    })();

    // You CAN create a closure with unsized arguments,
    // but you CANNOT call it.
    // This is an implementation detail and may be changed in the future.
    let f = |x: dyn Any| {};
    f(*answer());  // ERROR
}