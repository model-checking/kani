// run-rustfix
#![warn(clippy::single_component_path_imports)]
#![allow(unused_imports)]

// #7106: use statements exporting a macro within a crate should not trigger lint

macro_rules! m1 {
    () => {};
}
pub(crate) use m1; // ok

macro_rules! m2 {
    () => {};
}
use m2; // fail

fn main() {
    m1!();
    m2!();
}
