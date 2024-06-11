// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    // declare pointer to integer
    let p_subscoped: *const u32;

    {
        // declare integer within subscope
        let a = 7;
        // create pointer to that integer
        p_subscoped = &a;
        // assert pointer is currently correct
        unsafe {
            assert!(*p_subscoped == 7);
        }
    }
    // p_subscoped is now pointing to undefined memory

    // dereferencing pointers to undefined memory
    unsafe {
        assert!(*p_subscoped == 7);
    }
}
