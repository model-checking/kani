// compile-flags: --edition 2015
#![allow(unused)]
mod foo {
    pub mod example { pub mod iter {} }
    pub mod baz { pub fn foobaz() {} }
}
use foo::example::iter;
use ::foo::baz::foobaz;
fn main() {}