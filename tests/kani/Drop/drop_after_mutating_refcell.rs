// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks whether dropping after mutating with Rc<Refcell<>>
// is handled correctly.
//
// Note: If you were to use Rc<RefCell<dyn CELLValueInFuture>>, then
// kani will fail with an unsupported feature error. This is because
// RefCell uses UnsafeCell inside, and that is not entirely supported
// as of kani 0.4.0.

use std::cell::RefCell;
use std::rc::Rc;

static mut CELL: i32 = 0;

trait CELLValueInFuture {
    fn set_inner_value(&mut self, new_value: i32);
    fn get_inner_value(&self) -> i32;
}

struct DropSetCELLToInner {
    set_cell_to: i32,
}

impl CELLValueInFuture for DropSetCELLToInner {
    fn set_inner_value(&mut self, new_value: i32) {
        self.set_cell_to = new_value;
    }

    fn get_inner_value(&self) -> i32 {
        self.set_cell_to
    }
}

impl Drop for DropSetCELLToInner {
    fn drop(&mut self) {
        unsafe {
            CELL = self.get_inner_value();
        }
    }
}

#[kani::proof]
fn main() {
    {
        let set_to_one = DropSetCELLToInner { set_cell_to: 1 };
        let wrapped_drop: Rc<RefCell<DropSetCELLToInner>> = Rc::new(RefCell::new(set_to_one));

        wrapped_drop.borrow_mut().set_inner_value(2);
    }
    assert_eq!(unsafe { CELL }, 2, "Drop should be called. New value used during drop.");
}
