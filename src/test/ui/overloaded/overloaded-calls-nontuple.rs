#![feature(fn_traits, unboxed_closures)]

use std::ops::FnMut;

struct S {
    x: isize,
    y: isize,
}

impl FnMut<isize> for S {
    extern "rust-call" fn call_mut(&mut self, z: isize) -> isize {
        self.x + self.y + z
    }
    //~^^^ ERROR functions with the "rust-call" ABI must take a single non-self argument
}

impl FnOnce<isize> for S {
    type Output = isize;
    extern "rust-call" fn call_once(mut self, z: isize) -> isize { self.call_mut(z) }
    //~^ ERROR functions with the "rust-call" ABI must take a single non-self argument
}

fn main() {
    let mut s = S {
        x: 1,
        y: 2,
    };
    drop(s(3))  //~ ERROR cannot use call notation
}
