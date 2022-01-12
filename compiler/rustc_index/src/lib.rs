#![feature(allow_internal_unstable)]
#![feature(bench_black_box)]
#![feature(extend_one)]
#![feature(min_specialization)]
#![feature(step_trait)]
#![feature(test)]
#![feature(let_else)]

pub mod bit_set;
pub mod interval;
pub mod vec;

// FIXME(#56935): Work around ICEs during cross-compilation.
#[allow(unused)]
extern crate rustc_macros;
