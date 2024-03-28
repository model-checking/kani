// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
mod globals {
    pub static STATIC_VAR: i32 = 5;
    pub static mut STATIC_MUT_VAR: i32 = 7;
}
let local_var = 3;
local_var;
globals::STATIC_VAR;
unsafe { globals::STATIC_MUT_VAR };
let some_constructor = Some::<i32>;
let push_integer = Vec::<i32>::push;
let slice_reverse = <[i32]>::reverse;
}