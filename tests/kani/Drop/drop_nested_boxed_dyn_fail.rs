// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-verify-fail
// cbmc-flags: --unwinding-assertions

static mut CELL: i32 = 0;

struct Concrete;

impl Drop for Concrete {
    fn drop(&mut self) {
        unsafe {
            CELL += 1;
        }
    }
}

#[kani::proof]
#[kani::unwind(2)]
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
        kani::expect_fail(CELL == 2, "wrong cell value"); // Should fail
    }
}
