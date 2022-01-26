// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
trait WeirdAdd {
    fn add(&self, rhs: i32) -> i32;
}

impl WeirdAdd for i32 {
    fn add(&self, other: i32) -> i32 {
        self / 2 + other / 2
    }
}

fn std_add(x: i32, y: i32) -> i32 {
    x + y
}

fn weird_add(x: &dyn WeirdAdd, y: i32) -> i32 {
    x.add(y)
}

fn main() {
    let x = 2;
    let y = 4;

    let std_add = std_add(x, y);
    assert!(std_add == 6);
    let weird_add = weird_add(&x as &dyn WeirdAdd, y);
    assert!(weird_add == 3);
}
