// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles simple trait impl

trait T {
    fn get_val(&self) -> i32;
}
struct A {
    val: i32,
}

struct B {
    index: i32,
}

impl T for A {
    fn get_val(&self) -> i32 {
        self.val
    }
}

impl T for B {
    fn get_val(&self) -> i32 {
        self.index
    }
}

#[kani::proof]
fn main() {
    let e = A { val: 3 };
    let k = B { index: 3 };
    let i = e.get_val();
    let j = k.get_val();
}
