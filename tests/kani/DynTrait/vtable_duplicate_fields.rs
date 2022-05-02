// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
trait A {
    fn foo(&self) -> i32;
}

trait B {
    fn foo(&self) -> i32;
}

trait T: A + B {}

struct S {
    x: i32,
    y: i32,
}

impl S {
    fn new(a: i32, b: i32) -> S {
        S { x: a, y: b }
    }
    fn new_box(a: i32, b: i32) -> Box<dyn T> {
        Box::new(S::new(a, b))
    }
}

impl A for S {
    fn foo(&self) -> i32 {
        self.x
    }
}

impl B for S {
    fn foo(&self) -> i32 {
        self.y
    }
}

impl T for S {}

#[kani::proof]
fn main() {
    let t = S::new_box(1, 2);
    let a = <dyn T as A>::foo(&*t);
    assert!(a == 1);
    let b = <dyn T as B>::foo(&*t);
    assert!(b == 2);
}
