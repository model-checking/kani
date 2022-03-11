// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check drop implementation for a nested boxed dynamic trait objects.
// There is an implicit self-recursive call to drop_in_place, so we
// need to set an unwind bound.

// Temporarily disabling assertion reachability checks because they trigger a
// crash in CBMC:
// https://github.com/diffblue/cbmc/issues/6691
// https://github.com/model-checking/kani/issues/861
// kani-flags: --no-assertion-reach-checks

// cbmc-flags: --unwind 2 --unwinding-assertions

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
        assert!(CELL == 1);
    }
}
