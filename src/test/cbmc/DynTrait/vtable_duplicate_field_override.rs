// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
trait A {
    fn foo(&self) -> i32;
}

trait B {
    fn foo(&self) -> i32;
}

trait T: A + B {
    fn foo(&self) -> i32;
}

struct S {
    x: i32,
    y: i32,
    z: i32,
}

impl S {
    fn new(a: i32, b: i32, c: i32) -> S {
        S { x: a, y: b, z: c }
    }
    fn new_box(a: i32, b: i32, c: i32) -> Box<dyn T> {
        Box::new(S::new(a, b, c))
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

impl T for S {
    fn foo(&self) -> i32 {
        self.z
    }
}

fn main() {
    let t = S::new_box(1, 2, 3);
    let a = <dyn T as A>::foo(&*t);
    assert!(a == 1);
    let b = <dyn T as B>::foo(&*t);
    assert!(b == 2);
    let t_value = T::foo(&*t);
    assert!(t_value == 3);
}
