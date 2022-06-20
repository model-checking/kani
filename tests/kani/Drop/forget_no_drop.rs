// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Tests the property that if you let memory leak via
//! std::mem::forget, drop will not be called.

static mut CELL: i32 = 0;

struct IncrementCELLWhenDropped;
impl Drop for IncrementCELLWhenDropped {
    fn drop(&mut self) {
        unsafe {
            CELL = 1;
        }
    }
}

#[kani::proof]
fn main() {
    {
        let x1 = IncrementCELLWhenDropped {};
        std::mem::forget(x1);
    }
    unsafe {
        assert!(CELL == 0);
    }
}
