// run-rustfix
// rustfix-only-machine-applicable
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_must_use)]

fn foo() -> i32 {
    {2} + {2} //~ ERROR expected expression, found `+`
    //~^ ERROR mismatched types
}

fn bar() -> i32 {
    {2} + 2 //~ ERROR leading `+` is not supported
    //~^ ERROR mismatched types
}

fn zul() -> u32 {
    let foo = 3;
    { 42 } + foo; //~ ERROR expected expression, found `+`
    //~^ ERROR mismatched types
    32
}

fn baz() -> i32 {
    { 3 } * 3 //~ ERROR type `{integer}` cannot be dereferenced
    //~^ ERROR mismatched types
}

fn moo(x: u32) -> bool {
    match x {
        _ => 1,
    } > 0 //~ ERROR expected expression
}

fn qux() -> u32 {
    {2} - 2 //~ ERROR cannot apply unary operator `-` to type `u32`
    //~^ ERROR mismatched types
}

fn main() {}
