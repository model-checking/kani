// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[derive(Copy, Clone)]
struct TestStruct {
    x: i64,
    y: i32,
}

static mut X: TestStruct = TestStruct { x: 12, y: 14 };

fn foo() -> TestStruct {
    unsafe { X }
}

fn mutate_the_thing(nx: i64, ny: i32) {
    unsafe {
        X.x = nx;
        X.y = ny;
    }
}

fn main() {
    assert!(foo().x == 12);
    assert!(foo().y == 12);
    assert!(foo().x == 14);
    assert!(foo().y == 14);

    mutate_the_thing(1, 2);
    assert!(foo().x == 1);
    assert!(foo().y == 1);
    assert!(foo().x == 2);
    assert!(foo().y == 2);

    mutate_the_thing(1 << 62, 1 << 31);
    assert!(foo().x == 1 << 62);
    assert!(foo().x == 1 << 31);
    assert!(foo().y == 1 << 31);
}
