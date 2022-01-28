// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check drop implementation for a concrete, non-trait object.

static mut CELL: i32 = 0;

struct Concrete1;

impl Drop for Concrete1 {
    fn drop(&mut self) {
        unsafe {
            CELL = 1;
        }
    }
}

fn main() {
    {
        let _x1 = Concrete1 {};
    }
    unsafe {
        assert!(CELL == 1);
    }
}
