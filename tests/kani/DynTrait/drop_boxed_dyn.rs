// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check drop implementation for a boxed dynamic trait object.

static mut CELL: i32 = 0;

trait T {
    fn t(&self) {}
}

struct Concrete1;

impl T for Concrete1 {}

impl Drop for Concrete1 {
    fn drop(&mut self) {
        unsafe {
            CELL = 1;
        }
    }
}

struct Concrete2;

impl T for Concrete2 {}

impl Drop for Concrete2 {
    fn drop(&mut self) {
        unsafe {
            CELL = 2;
        }
    }
}

#[kani::proof]
fn main() {
    {
        let x: Box<dyn T>;
        if kani::any() {
            x = Box::new(Concrete1 {});
        } else {
            x = Box::new(Concrete2 {});
        }
    }
    unsafe {
        assert!(CELL == 1 || CELL == 2);
    }
}
