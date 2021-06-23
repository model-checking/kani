// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
static mut X: i32 = 12;

fn foo() -> i32 {
    unsafe { X }
}

fn mutate_the_thing(new: i32) {
    unsafe {
        X = new;
    }
}

fn main() {
    assert!(10 == foo());
    assert!(12 == foo());
    mutate_the_thing(10);
    assert!(10 == foo());
    assert!(12 == foo());
}
