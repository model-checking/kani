// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check drop implementation for a nested boxed dynamic trait objects.
// There is an implicit self-recursive call to drop_in_place, so we
// need to set an unwind bound.

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
        assert!(CELL == 1);
    }
}
