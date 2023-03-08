// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks the performance of pushing onto a vector of Box<dyn _>.
//! The test is from https://github.com/model-checking/kani/issues/1657.
//! Pre CBMC 5.71.0, it took 3 minutes and consumed more than 14 GB of memory.
//! With CBMC 5.71.0, it takes ~3 seconds and consumes ~150 MB of memory.

const N: usize = 4;
const M: usize = N + 1;

trait T {
    fn foo(&self) -> i32;
}

struct A {
    x: i32,
}

impl T for A {
    fn foo(&self) -> i32 {
        self.x
    }
}

struct B {
    x: i32,
}

impl T for B {
    fn foo(&self) -> i32 {
        self.x
    }
}

#[kani::proof]
#[kani::unwind(6)]
fn main() {
    let mut a: Vec<Box<dyn T>> = Vec::new();
    a.push(Box::new(A { x: 9 }));
    for i in 1..N {
        a.push(Box::new(B { x: 9 }));
    }
    let mut val: i32 = 0;
    for _i in 0..M {
        let index: usize = kani::any();
        kani::assume(index < a.len());
        let x = a[index].as_mut().foo();
        val += x;
    }
    assert_eq!(val as usize, 9 * M);
}
