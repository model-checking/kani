// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// rmc-verify-fail
// cbmc-flags: --unwind 2 --unwinding-assertions

include!("../../rmc-prelude.rs");

static mut CELL: i32 = 0;

struct Concrete;

impl Drop for Concrete {
    fn drop(&mut self) {
        unsafe {
            CELL += 1;
        }
    }
}

fn main() {
    // Check normal box
    {
        let _x: Box<dyn Send> = Box::new(Concrete {});
    }
    unsafe {
        assert!(CELL == 1);
    }

    // Reset global
    unsafe {
        CELL = 0;
    }

    // Check nested box, still only incremented once
    {
        let x: Box<dyn Send> = Box::new(Concrete {});
        let _nested: Box<dyn Send> = Box::new(x);
    }
    unsafe {
        __VERIFIER_expect_fail(CELL == 2, "wrong cell value"); // Should fail
    }
}
