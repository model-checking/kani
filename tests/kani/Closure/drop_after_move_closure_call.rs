// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that structs moved into Box<Fn> are dropped when
// the function is dropped.

static mut CELL: i32 = 0;

struct DropIncrementCELLByOne {}

impl DropIncrementCELLByOne {
    fn do_nothing(&self) {}
}

impl Drop for DropIncrementCELLByOne {
    fn drop(&mut self) {
        unsafe {
            CELL += 1;
        }
    }
}

#[kani::proof]
fn main() {
    {
        let object_to_drop = DropIncrementCELLByOne {};
        let fun: Box<dyn FnOnce() -> ()> = Box::new(move || {
            object_to_drop.do_nothing();
        });

        fun();
    }
    assert_eq!(unsafe { CELL }, 1, "Drop should be called when move fn is used");

    {
        let object_to_drop = DropIncrementCELLByOne {};
        let _fun: Box<dyn FnOnce() -> ()> = Box::new(move || {
            object_to_drop.do_nothing();
        });
    }
    assert_eq!(unsafe { CELL }, 2, "Drop should still be called when move fn is not used");
}
