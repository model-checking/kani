// Regression test for issue #91210.

// run-rustfix

#![allow(unused)]

struct Foo { read: i32 }

unsafe fn blah(x: *mut Foo) {
    x.read = 4;
    //~^ ERROR: attempted to take value of method
    //~| HELP: to access the field, dereference first
}

fn main() {}
